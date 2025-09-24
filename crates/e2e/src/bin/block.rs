use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::{NUM_BLOCKS, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let _proof = prove_blocks(
        TRUSTED_HEIGHT,
        NUM_BLOCKS,
        &mut FixedBytes::from_hex(TRUSTED_ROOT).unwrap(),
    )
    .await
    .unwrap();
    println!("Proof generated successfully!");
}
