use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc::GrpcClient;
use celestia_grpc_client::{MsgSubmitMessages, MsgUpdateZkExecutionIsm};
use celestia_types::hash::Hash;
use e2e::{
    config::{EV_RPC, TARGET_HEIGHT},
    prover::message::prove_messages,
};
use e2e::{
    config::{NUM_BLOCKS, START_HEIGHT, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};
use ev_state_queries::MockStateQueryProvider;
use ev_zkevm_types::programs::block::BlockRangeExecOutput;
use sp1_sdk::{EnvProver, ProverClient};
use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use url::Url;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();

    // 1. submit block proof message
    let grpc_client = GrpcClient::builder()
        .url("http://localhost:9090")
        .private_key_hex("6e30efb1d3ebd30d1ba08c8d5fc9b190e08394009dc1dd787a69e60c33288a8c")
        .build()
        .unwrap();

    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let tx_config = celestia_grpc::TxConfig {
        gas_limit: Some(200000u64),
        gas_price: Some(1000_f64),
        memo: Some("zkISM state transition proof submission".to_string()),
        ..Default::default()
    };
    let block_proof = prove_blocks(
        START_HEIGHT,
        TRUSTED_HEIGHT,
        NUM_BLOCKS,
        &mut FixedBytes::from_hex(TRUSTED_ROOT).unwrap(),
        client.clone(),
    )
    .await
    .expect("Failed to prove blocks");

    let celestia_target_height = START_HEIGHT + NUM_BLOCKS - 1;

    let block_proof_msg = MsgUpdateZkExecutionIsm::new(
        "0x726f757465725f69736d000000000000000000000000002a0000000000000000".to_string(),
        celestia_target_height,
        block_proof.bytes(),
        block_proof.public_values.as_slice().to_vec(),
        "celestia1y3kf30y9zprqzr2g2gjjkw3wls0a35pfs3a58q".to_string(),
    );

    match grpc_client.submit_message(block_proof_msg, tx_config.clone()).await {
        Ok(tx_info) => {
            println!(
                "Successfully submitted state transition proof: tx_hash={}, height={}",
                tx_info.hash,
                tx_info.height.value()
            );
            wait_for_tx(&grpc_client, tx_info.hash).await.unwrap();
        }
        Err(e) => {
            panic!("Failed to submit state transition proof: {e}");
        }
    }

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
        "0x726f757465725f69736d000000000000000000000000002a0000000000000000".to_string(),
        TARGET_HEIGHT,
        message_proof.bytes(),
        message_proof.public_values.as_slice().to_vec(),
        "celestia1y3kf30y9zprqzr2g2gjjkw3wls0a35pfs3a58q".to_string(),
    );

    // 2. submit message proof message
    match grpc_client.submit_message(message_proof_msg, tx_config).await {
        Ok(tx_info) => {
            println!(
                "Successfully submitted message proof: tx_hash={}, height={}",
                tx_info.hash,
                tx_info.height.value()
            );
            wait_for_tx(&grpc_client, tx_info.hash).await.unwrap();
        }
        Err(e) => {
            panic!("Failed to submit message proof: {e}");
        }
    }
}

async fn wait_for_tx(grpc_client: &GrpcClient, tx_hash: Hash) -> anyhow::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let mut attempts = 0;
        match grpc_client.get_tx(tx_hash).await {
            Ok(tx) => {
                println!("Tx {} succeeded! Response: {:?}", tx_hash, tx.tx_response);
                return Ok(());
            }
            Err(e) => {
                println!("Tx {tx_hash} not found on chain: {e:?}");
            }
        }

        attempts += 1;
        if attempts > 12 {
            return Err(anyhow::anyhow!("Timeout waiting for tx {tx_hash:?}"));
        }
        sleep(Duration::from_secs(5)).await;
    }

    Err(anyhow::anyhow!("Timeout waiting for tx {tx_hash:?}"))
}

/*
I use this command

grpcurl -plaintext -d '{"key": "rhb/2000/d"}' localhost:7331 evnode.v1.StoreService.GetMetadata

to figure out the celestia height for an evm block. The result is the base64 little-endian encoded celestia height.
*/
