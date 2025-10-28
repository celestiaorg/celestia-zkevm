use std::{env, sync::Arc};

use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::debug::{TARGET_HEIGHT, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};
use ev_prover::inclusion_height;
use sp1_sdk::ProverClient;

#[tokio::main]
async fn main() {
    let sequencer_rpc_url = env::var("SEQUENCER_RPC_URL").unwrap();
    dotenvy::dotenv().ok();
    let trusted_inclusion_height = inclusion_height(TRUSTED_HEIGHT, sequencer_rpc_url.clone())
        .await
        .unwrap()
        + 1;
    let target_inclusion_height = inclusion_height(TARGET_HEIGHT, sequencer_rpc_url).await.unwrap();
    let num_blocks = target_inclusion_height - trusted_inclusion_height + 1;
    let client = Arc::new(ProverClient::from_env());
    prove_blocks(
        trusted_inclusion_height,
        TRUSTED_HEIGHT,
        num_blocks,
        &mut FixedBytes::from_hex(TRUSTED_ROOT).unwrap(),
        client,
    )
    .await
    .expect("Failed to prove blocks");
}
