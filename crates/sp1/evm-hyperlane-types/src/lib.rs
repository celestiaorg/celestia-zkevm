use std::str::FromStr;

use alloy_primitives::Address;
use evm_state_types::hyperlane::HyperlaneMessage;
use evm_storage_proofs::types::{HYPERLANE_MERKLE_TREE_KEYS, HyperlaneBranchProofInputs};
use serde::{Deserialize, Serialize};
pub mod tree;
use tree::MerkleTree;

use crate::tree::ZERO_BYTES;

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Inputs for the hyperlane message circuit.
pub struct HyperlaneMessageInputs {
    pub state_root: String,
    pub contract: String,
    pub messages: Vec<HyperlaneMessage>,
    pub branch_proof: HyperlaneBranchProofInputs,
    pub snapshot: MerkleTree,
}

/// Implementation of the hyperlane message inputs.
impl HyperlaneMessageInputs {
    pub fn new(
        state_root: String,
        contract: String,
        messages: Vec<HyperlaneMessage>,
        branch_proof: HyperlaneBranchProofInputs,
        snapshot: MerkleTree,
    ) -> Self {
        Self {
            state_root,
            contract,
            messages,
            branch_proof,
            snapshot,
        }
    }

    /// Verify the hyperlane message inputs against the branch proof and snapshot.
    pub fn verify(&mut self) {
        let message_ids: Vec<String> = self.messages.iter().map(|m| m.id()).collect();
        for message_id in message_ids {
            self.snapshot
                .insert(message_id)
                .expect("Failed to insert message id into snapshot");
        }

        // sanity check, we can't prove an empty hyperlane tree against state_root
        if self
            .snapshot
            .branch
            .iter()
            .all(|_| self.snapshot.branch.iter().all(|b| b == ZERO_BYTES))
        {
            panic!("Snapshot branch is empty (all zero bytes) before proof verification");
        }

        for idx in 0..HYPERLANE_MERKLE_TREE_KEYS.len() {
            // The branch nodes of the snapshot after insert must match the branch nodes of the incremental
            // tree on the EVM chain.
            assert_eq!(self.snapshot.branch[idx], self.branch_proof.get_branch_node(idx));
        }

        let verified = self
            .branch_proof
            .verify(
                &HYPERLANE_MERKLE_TREE_KEYS,
                Address::from_str(&self.contract).unwrap(),
                &self.state_root,
            )
            .expect("Failed to verify branch proof");
        assert!(verified);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HyperlaneMessageOutputs {
    pub state_root: String,
    // todo: output just the id, not the entire message
    pub message_ids: Vec<String>,
}

impl HyperlaneMessageOutputs {
    pub fn new(state_root: String, message_ids: Vec<String>) -> Self {
        Self {
            state_root,
            message_ids,
        }
    }
}
