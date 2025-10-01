use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::{NUM_BLOCKS, START_HEIGHT, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    prove_blocks(
        START_HEIGHT,
        TRUSTED_HEIGHT,
        NUM_BLOCKS,
        &mut FixedBytes::from_hex(TRUSTED_ROOT).unwrap(),
    )
    .await
    .expect("Failed to prove blocks");
}
