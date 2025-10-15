use alloy_provider::ProviderBuilder;
use e2e::{
    config::debug::{EV_RPC, TARGET_HEIGHT},
    prover::message::prove_messages,
};
use ev_state_queries::MockStateQueryProvider;
use sp1_sdk::{EnvProver, ProverClient};
use std::{str::FromStr, sync::Arc};
use storage::hyperlane::snapshot::HyperlaneSnapshotStore;
use url::Url;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let snapshot_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("data")
        .join("snapshots.db");
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap());
    hyperlane_snapshot_store.reset_db().unwrap();
    let snapshot = hyperlane_snapshot_store.get_snapshot(0).unwrap();
    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());
    let _proof = prove_messages(
        0,
        TARGET_HEIGHT,
        &evm_provider.clone(),
        &MockStateQueryProvider::new(evm_provider),
        client,
        snapshot,
    )
    .await
    .unwrap();
    println!("Proof generated successfully!");
}
