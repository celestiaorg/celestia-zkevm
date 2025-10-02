use std::sync::Arc;

use alloy_primitives::{FixedBytes, hex::FromHex};
use e2e::{
    config::{TARGET_HEIGHT, TRUSTED_HEIGHT, TRUSTED_ROOT},
    prover::block::prove_blocks,
};
use ev_types::v1::{GetMetadataRequest, store_service_client::StoreServiceClient};
use sp1_sdk::ProverClient;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let trusted_inclusion_height = inclusion_height(TRUSTED_HEIGHT).await.unwrap() + 1;
    let target_inclusion_height = inclusion_height(TARGET_HEIGHT).await.unwrap();
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
