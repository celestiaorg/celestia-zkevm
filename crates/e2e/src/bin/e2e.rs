use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc_client::MsgRemoteTransfer;
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{
    MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm, QueryIsmRequest, client::CelestiaIsmClient,
};
use e2e::config::e2e::ISM_ID;
use e2e::config::other::EV_RPC;
use e2e::prover::block::prove_blocks;
use e2e::prover::helpers::transfer_back;
use e2e::prover::message::prove_messages;
use ev_state_queries::MockStateQueryProvider;
use ev_types::v1::{GetMetadataRequest, store_service_client::StoreServiceClient};
use ev_zkevm_types::hyperlane::encode_hyperlane_message;
use sp1_sdk::{EnvProver, ProverClient};
use std::time::Duration;
use std::{str::FromStr, sync::Arc};
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

// todo: find a better way to wait for the balance to be updated
// ideally deterministically check if the hyperlane transfer was finalized on the destination
// and wait until the ev block was included in a Celestia block when going back
const MAX_RETRIES: u64 = 10;
const RETRY_DELAY: u64 = 2;

#[tokio::main]
#[allow(clippy::field_reassign_with_default)]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();
    let mut filter = EnvFilter::new("sp1_core=warn,sp1_runtime=warn,sp1_sdk=warn,sp1_vm=warn");
    if let Ok(env_filter) = std::env::var("RUST_LOG") {
        if let Ok(parsed) = env_filter.parse() {
            filter = filter.add_directive(parsed);
        }
    }
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // instantiate ISM client for submitting payloads and querying state
    let ism_client = CelestiaIsmClient::from_env().await.unwrap();
    let resp = ism_client
        .ism(QueryIsmRequest { id: ISM_ID.to_string() })
        .await
        .unwrap();

    let ism = resp.ism.expect("ZKISM not found");
    let trusted_root_hex = alloy::hex::encode(ism.state_root);
    let trusted_height = ism.height;

    // manually override the signer address and private key for the transfer
    let mut celestia_client_config = ClientConfig::default();
    celestia_client_config.private_key_hex =
        "f7ec3cfaa1ae36c9c907d5ed5397503fc6e9f26cb69bfd83dbe45c5b2a717021".to_string();
    celestia_client_config.signer_address = "celestia1d2qfkdk27r2x4y67ua5r2pj7ck5t8n4890x9wy".to_string();
    let ism_client = CelestiaIsmClient::new(celestia_client_config).await.unwrap();
    let transfer_msg = MsgRemoteTransfer::new(
        ism_client.signer_address().to_string(),
        "0x726f757465725f61707000000000000000000000000000010000000000000000".to_string(),
        1234,
        "0x000000000000000000000000aF9053bB6c4346381C77C2FeD279B17ABAfCDf4d".to_string(),
        "1000".to_string(),
    );

    info!("Bridging Tia from Celestia to Evolve...");
    let response = ism_client.send_tx(transfer_msg).await.unwrap();
    assert!(response.success);

    info!("Waiting for Evolve balance to be updated...");

    // next trigger make transfer-back
    info!("Submitting Hyperlane deposit message on Evolve...");
    let mut retries = 0;
    let target_height = {
        loop {
            let target_height = match transfer_back().await {
                Ok(height) => height,
                Err(_) => {
                    if retries > MAX_RETRIES {
                        panic!("Failed to get target height after {} retries", MAX_RETRIES);
                    }
                    sleep(Duration::from_secs(RETRY_DELAY)).await;
                    retries += 1;
                    continue;
                }
            };
            break target_height;
        }
    };
    info!("[Done] submitting transfer Messages");
    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());

    let mut retries = 0;
    let target_inclusion_height = {
        loop {
            let target_inclusion_height = match inclusion_height(target_height).await {
                Ok(height) => height,
                Err(_) => {
                    if retries > MAX_RETRIES {
                        panic!("Failed to get target inclusion height after {} retries", MAX_RETRIES);
                    }
                    sleep(Duration::from_secs(RETRY_DELAY)).await;
                    retries += 1;
                    continue;
                }
            };
            break target_inclusion_height;
        }
    };
    let start_height = inclusion_height(trusted_height).await.unwrap() + 1;
    let num_blocks = target_inclusion_height - start_height;

    info!("Proving Evolve blocks...");
    let block_proof = prove_blocks(
        start_height,
        trusted_height,
        num_blocks,
        &mut FixedBytes::from_hex(trusted_root_hex).unwrap(),
        client.clone(),
    )
    .await
    .expect("Failed to prove blocks");
    info!("[Done] proving blocks");

    let block_proof_msg = MsgUpdateZkExecutionIsm::new(
        ISM_ID.to_string(),
        target_inclusion_height,
        block_proof.bytes(),
        block_proof.public_values.as_slice().to_vec(),
        ism_client.signer_address().to_string(),
    );

    info!("Updating ZKISM on Celestia...");
    let response = ism_client.send_tx(block_proof_msg).await.unwrap();
    assert!(response.success);
    info!("[Done] ZKISM was updated successfully");

    let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());
    info!("Proving Evolve Hyperlane deposit events...");
    let message_proof = prove_messages(
        target_height,
        &evm_provider.clone(),
        &MockStateQueryProvider::new(evm_provider),
        client.clone(),
    )
    .await
    .unwrap();

    let message_proof_msg = MsgSubmitMessages::new(
        ISM_ID.to_string(),
        target_height,
        message_proof.0.bytes(),
        message_proof.0.public_values.as_slice().to_vec(),
        ism_client.signer_address().to_string(),
    );
    info!("[Done] ZKISM was updated successfully");

    info!("Submitting Hyperlane tree proof to ZKISM...");
    let response = ism_client.send_tx(message_proof_msg).await.unwrap();
    assert!(response.success);
    info!("[Done] ZKISM was updated successfully");

    info!("Relaying verified Hyperlane messages to Celestia...");
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
    info!("[Done] Tia was bridged back to Celestia");
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
