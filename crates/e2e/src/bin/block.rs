use std::sync::Arc;

use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::{NUM_BLOCKS, START_HEIGHT, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};
use sp1_sdk::ProverClient;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let client = Arc::new(ProverClient::from_env());
    prove_blocks(
        START_HEIGHT,
        TRUSTED_HEIGHT,
        NUM_BLOCKS,
        &mut FixedBytes::from_hex(TRUSTED_ROOT).unwrap(),
        client,
    )
    .await
    .expect("Failed to prove blocks");
}
