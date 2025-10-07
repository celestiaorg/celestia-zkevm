use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use ev_prover::prover::programs::message::{AppContext, HyperlaneMessageProver, MerkleTreeState};
use ev_state_queries::{DefaultProvider, MockStateQueryProvider};
use reqwest::Url;
use storage::{
    hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore},
    proofs::RocksDbProofStorage,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_run_message_prover() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let tmp = TempDir::new().expect("cannot create temp directory");
    let snapshot_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(&tmp)
        .join("data")
        .join("snapshots.db");
    let message_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(&tmp)
        .join("data")
        .join("messages.db");
    let proof_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(&tmp)
        .join("data")
        .join("proofs.db");
    let hyperlane_message_store = Arc::new(HyperlaneMessageStore::new(message_storage_path).unwrap());
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap());
    let proof_store = Arc::new(RocksDbProofStorage::new(proof_storage_path).unwrap());

    hyperlane_message_store.prune_all().unwrap();
    hyperlane_snapshot_store.prune_all().unwrap();

    let app = AppContext {
        evm_rpc: "http://127.0.0.1:8545".to_string(),
        evm_ws: "ws://127.0.0.1:8546".to_string(),
        mailbox_address: Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap(),
        merkle_tree_address: Address::from_str("0xfcb1d485ef46344029d9e8a7925925e146b3430e").unwrap(),
        merkle_tree_state: RwLock::new(MerkleTreeState::new(0, 0)),
    };

    let evm_provider: DefaultProvider =
        ProviderBuilder::new().connect_http(Url::from_str("http://127.0.0.1:8545").unwrap());

    let prover = HyperlaneMessageProver::new(
        app,
        hyperlane_message_store,
        hyperlane_snapshot_store,
        proof_store,
        Arc::new(MockStateQueryProvider::new(evm_provider)),
    )
    .unwrap();
    prover.run().await.unwrap();
}
