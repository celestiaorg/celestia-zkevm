use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::{TARGET_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let _proof = prove_blocks(0, 1, FixedBytes::from_hex(TRUSTED_ROOT).unwrap(), TARGET_HEIGHT)
        .await
        .unwrap();
    println!("Proof generated successfully!");
}
