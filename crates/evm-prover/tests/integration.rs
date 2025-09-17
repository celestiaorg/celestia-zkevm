use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use alloy_primitives::Address;
use evm_prover::prover::programs::message::{AppContext, HyperlaneMessageProver, MerkleTreeState};
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};

#[tokio::test]
async fn test_run_message_prover() {
    #[allow(unused_imports)]
    let hyperlane_message_store =
        Arc::new(HyperlaneMessageStore::from_path_relative(2, storage::hyperlane::message::IndexMode::Block).unwrap());
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::from_path_relative(2).unwrap());
    hyperlane_message_store.prune_all().unwrap();
    hyperlane_snapshot_store.prune_all().unwrap();

    let app = AppContext {
        celestia_rpc: "http://127.0.0.1:26657".to_string(),
        evm_rpc: "http://127.0.0.1:8545".to_string(),
        evm_ws: "ws://127.0.0.1:8546".to_string(),
        mailbox_address: Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap(),
        merkle_tree_address: Address::from_str("0xfcb1d485ef46344029d9e8a7925925e146b3430e").unwrap(),
        trusted_state: RwLock::new(MerkleTreeState::new(0, 0)),
    };

    let prover = HyperlaneMessageProver::new(app, hyperlane_message_store, hyperlane_snapshot_store).unwrap();
    prover.run().await.unwrap();
}
