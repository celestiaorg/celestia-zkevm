use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{
    MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm, QueryIsmRequest, client::CelestiaIsmClient,
};
use e2e::config::e2e::ISM_ID;
use e2e::config::other::EV_RPC;
use e2e::prover::message::prove_messages;
use e2e::{config::e2e::TARGET_HEIGHT, prover::block::prove_blocks};
use ev_state_queries::MockStateQueryProvider;
use ev_types::v1::{GetMetadataRequest, store_service_client::StoreServiceClient};
use ev_zkevm_types::hyperlane::encode_hyperlane_message;
use sp1_sdk::{EnvProver, ProverClient};
use std::{str::FromStr, sync::Arc};
use url::Url;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();

    // instantiate ISM client for submitting payloads and querying state
    let config = ClientConfig::from_env().expect("failed to create celestia client config");
    let ism_client = CelestiaIsmClient::new(config).await.unwrap();

    let resp = ism_client
        .ism(QueryIsmRequest { id: ISM_ID.to_string() })
        .await
        .unwrap();

    let ism = resp.ism.expect("ZKISM not found");
    let trusted_root_hex = alloy::hex::encode(ism.state_root);
    let trusted_height = ism.height;

    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let target_inclusion_height = inclusion_height(TARGET_HEIGHT).await.unwrap();
    let start_height = inclusion_height(trusted_height).await.unwrap() + 1;
    let num_blocks = target_inclusion_height - start_height;
    let block_proof = prove_blocks(
        start_height,
        trusted_height,
        num_blocks,
        &mut FixedBytes::from_hex(trusted_root_hex).unwrap(),
        client.clone(),
    )
    .await
    .expect("Failed to prove blocks");

    let block_proof_msg = MsgUpdateZkExecutionIsm::new(
        ISM_ID.to_string(),
        target_inclusion_height,
        block_proof.bytes(),
        block_proof.public_values.as_slice().to_vec(),
        ism_client.signer_address().to_string(),
    );

    let response = ism_client.send_tx(block_proof_msg).await.unwrap();
    assert!(response.success);

    let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());
    let message_proof = prove_messages(
        TARGET_HEIGHT,
        &evm_provider.clone(),
        &MockStateQueryProvider::new(evm_provider),
        client.clone(),
    )
    .await
    .unwrap();

    let message_proof_msg = MsgSubmitMessages::new(
        ISM_ID.to_string(),
        TARGET_HEIGHT,
        message_proof.0.bytes(),
        message_proof.0.public_values.as_slice().to_vec(),
        ism_client.signer_address().to_string(),
    );

    let response = ism_client.send_tx(message_proof_msg).await.unwrap();
    assert!(response.success);

    // submit all now verified messages to hyperlane
    for message in message_proof.1 {
        let message_hex = alloy::hex::encode(encode_hyperlane_message(&message.message).unwrap());
        let msg = MsgProcessMessage::new(
            "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string(),
            ism_client.signer_address().to_string(),
            alloy::hex::encode(vec![]), // empty metadata; messages are pre-authorized before submission
            message_hex,
        );
        let response = ism_client.send_tx(msg).await.unwrap();
        assert!(response.success);
    }
}

// todo: find a place for this function and remove it from the binaries
async fn inclusion_height(block_number: u64) -> anyhow::Result<u64> {
    let mut client = StoreServiceClient::connect(e2e::config::e2e::SEQUENCER_URL).await?;
    let req = GetMetadataRequest {
        key: format!("rhb/{block_number}/d"),
    };

    let resp = client.get_metadata(req).await?;
    let height = u64::from_le_bytes(resp.into_inner().value[..8].try_into()?);

    Ok(height)
}
