// This endpoint generates a message proof for a new height,
// always starting with the original, empty Hyperlane Merkle Tree.

use std::{fs, str::FromStr, sync::Arc};

use alloy_primitives::{Address, FixedBytes, hex::FromHex};
use alloy_provider::Provider;
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use anyhow::Result;
use ev_state_queries::{DefaultProvider, StateQueryProvider};
use ev_zkevm_types::{
    events::{Dispatch, DispatchEvent},
    hyperlane::decode_hyperlane_message,
    programs::hyperlane::types::{
        HYPERLANE_MERKLE_TREE_KEYS, HyperlaneBranchProof, HyperlaneBranchProofInputs, HyperlaneMessageInputs,
    },
};
use sp1_sdk::{EnvProver, SP1ProofWithPublicValues, SP1Stdin};
use storage::hyperlane::{StoredHyperlaneMessage, message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use tempfile::TempDir;

use crate::config::other::{MAILBOX_ADDRESS, MERKLE_TREE_ADDRESS};

pub async fn prove_messages(
    target_height: u64,
    evm_provider: &DefaultProvider,
    state_query_provider: &dyn StateQueryProvider,
    client: Arc<EnvProver>,
) -> Result<(SP1ProofWithPublicValues, Vec<StoredHyperlaneMessage>)> {
    let tmp = TempDir::new().expect("cannot create temp directory");
    let state_root = state_query_provider
        .get_state_root(target_height)
        .await
        .expect("Failed to get state root");

    let merkle_proof = evm_provider
        .get_proof(
            Address::from_str(MERKLE_TREE_ADDRESS).unwrap(),
            HYPERLANE_MERKLE_TREE_KEYS
                .iter()
                .map(|k| FixedBytes::from_hex(k).unwrap())
                .collect(),
        )
        .block_id(target_height.into())
        .await?;
    let branch_proof = HyperlaneBranchProof::new(merkle_proof);

    // we need the message store for the indexer
    let message_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(&tmp)
        .join("data")
        .join("messages.db");
    let hyperlane_message_store = Arc::new(HyperlaneMessageStore::new(message_storage_path).unwrap());
    // prune in case non-empty
    hyperlane_message_store.prune_all().unwrap();

    let filter = Filter::new()
        .address(Address::from_str(MAILBOX_ADDRESS).unwrap())
        .event(&Dispatch::id())
        .from_block(0)
        .to_block(target_height);

    // run the indexer to get all messages that occurred since the last trusted height and insert them as if they all occurred at target_height
    let logs = evm_provider.get_logs(&filter).await?;
    for log in logs {
        match Dispatch::decode_log_data(log.data()) {
            Ok(event) => {
                let dispatch_event: DispatchEvent = event.into();
                let current_index = hyperlane_message_store.current_index()?;
                let hyperlane_message =
                    decode_hyperlane_message(&dispatch_event.message).expect("Failed to decode Hyperlane message");
                let stored_message = StoredHyperlaneMessage::new(hyperlane_message, Some(target_height));
                hyperlane_message_store
                    .insert_message(current_index, stored_message)
                    .unwrap();
                println!("Inserted Hyperlane Message at index: {current_index}");
            }
            Err(e) => {
                eprintln!("Failed to decode Dispatch Event: {e:?}");
            }
        }
    }

    // get all messages that were stored at target_height, note that this is very different to what we do in the
    // service, because here in the end to end we want to generate just one proof for everything that happened since block 0
    // the e2e is designed in such a way that it proves all messages that happened in the network every time it is run.
    let messages = hyperlane_message_store
        .get_by_block(target_height)
        .expect("Failed to get messages");

    // initialize and prune the snapshot store that will return the empty tree
    let snapshot_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(&tmp)
        .join("data")
        .join("snapshots.db");
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap());
    hyperlane_snapshot_store.prune_all().unwrap();
    let snapshot = hyperlane_snapshot_store.get_snapshot(0).unwrap();

    // Construct program inputs from values
    let input = HyperlaneMessageInputs::new(
        state_root.to_string(),
        MERKLE_TREE_ADDRESS.to_string(),
        messages.clone().into_iter().map(|m| m.message).collect(),
        HyperlaneBranchProofInputs::from(branch_proof),
        snapshot.clone(),
    );

    // generate and return the Groth16 proof
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);
    let ev_hyperlane_elf = fs::read("elfs/ev-hyperlane-elf").expect("Failed to read ELF");
    let (pk, vk) = client.setup(&ev_hyperlane_elf);
    let proof = client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");
    // could be removed but this is just a test so doesn't really matter
    // might actually be better to keep this in for sanity
    client.verify(&proof, &vk).expect("failed to verify proof");
    Ok((proof, messages))
}
