use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc_client::{
    MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm, ProofSubmitter, QueryIsmRequest,
    client::CelestiaIsmClient,
};
use e2e::prover::block::prove_blocks;
use e2e::{
    config::{EV_RPC, TARGET_HEIGHT},
    prover::message::prove_messages,
};
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
    let ism_client = CelestiaIsmClient::from_env().await.unwrap();

    let ism = ism_client
        .ism(QueryIsmRequest {
            id: "0x726f7465725f69736d00000000000000000000000000002a0000000000000001".to_string(),
        })
        .await
        .unwrap();

    let ism = ism.ism.expect("ZKISM not found");
    let ism_trusted_root_hex = alloy::hex::encode(ism.state_root);
    let ism_trusted_height = ism.height;

    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let trusted_inclusion_height = inclusion_height(ism_trusted_height).await.unwrap() + 1;
    let target_inclusion_height = inclusion_height(TARGET_HEIGHT).await.unwrap();
    let num_blocks = target_inclusion_height - trusted_inclusion_height + 1;
    let block_proof = prove_blocks(
        trusted_inclusion_height,
        ism_trusted_height,
        num_blocks,
        &mut FixedBytes::from_hex(ism_trusted_root_hex).unwrap(),
        client.clone(),
    )
    .await
    .expect("Failed to prove blocks");

    let block_proof_msg = MsgUpdateZkExecutionIsm::new(
        "0x726f757465725f69736d000000000000000000000000002a0000000000000001".to_string(),
        target_inclusion_height,
        block_proof.bytes(),
        block_proof.public_values.as_slice().to_vec(),
        "celestia1y3kf30y9zprqzr2g2gjjkw3wls0a35pfs3a58q".to_string(),
    );

    let response = ism_client.submit_state_transition_proof(block_proof_msg).await.unwrap();
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
        "0x726f757465725f69736d000000000000000000000000002a0000000000000001".to_string(),
        TARGET_HEIGHT,
        message_proof.0.bytes(),
        message_proof.0.public_values.as_slice().to_vec(),
        "celestia1y3kf30y9zprqzr2g2gjjkw3wls0a35pfs3a58q".to_string(),
    );

    let response = ism_client
        .submit_state_inclusion_proof(message_proof_msg)
        .await
        .unwrap();
    assert!(response.success);

    // submit all now verified messages to hyperlane
    for message in message_proof.1 {
        let message_hex = alloy::hex::encode(encode_hyperlane_message(&message.message).unwrap());
        let msg = MsgProcessMessage::new(
            "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string(),
            "celestia1y3kf30y9zprqzr2g2gjjkw3wls0a35pfs3a58q".to_string(),
            alloy::hex::encode(vec![]),
            message_hex,
        );
        let response = ism_client.process_hyperlane_message(msg).await.unwrap();
        assert!(response.success);
    }
}

// todo: find a place for this function and remove it from the binaries
async fn inclusion_height(block_number: u64) -> anyhow::Result<u64> {
    let mut client = StoreServiceClient::connect(e2e::config::SEQUENCER_URL).await?;
    let req = GetMetadataRequest {
        key: format!("rhb/{block_number}/d"),
    };

    let resp = client.get_metadata(req).await?;
    let height = u64::from_le_bytes(resp.into_inner().value[..8].try_into()?);

    Ok(height)
}
