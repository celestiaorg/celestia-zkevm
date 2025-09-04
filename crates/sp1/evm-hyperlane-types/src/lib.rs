use alloy_primitives::Address;
use evm_state_types::hyperlane::HyperlaneMessage;
use evm_storage_proofs::types::{HYPERLANE_MERKLE_TREE_KEYS, HyperlaneBranchProof};
use serde::{Deserialize, Serialize};
pub mod tree;
use tree::MerkleTree;

#[derive(Serialize, Deserialize)]
pub struct HyperlaneMessageInputs {
    pub state_root: String,
    pub contract: Address,
    pub messages: Vec<HyperlaneMessage>,
    pub branch_proof: HyperlaneBranchProof,
    pub snapshot: MerkleTree,
}

impl HyperlaneMessageInputs {
    pub fn new(
        state_root: String,
        contract: Address,
        messages: Vec<HyperlaneMessage>,
        branch_proof: HyperlaneBranchProof,
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
    pub fn verify(&mut self) {
        let message_ids: Vec<String> = self.messages.iter().map(|m| m.id()).collect();
        for message_id in message_ids {
            self.snapshot
                .insert(message_id)
                .expect("Failed to insert message id into snapshot");
        }
        self.branch_proof
            .verify(&HYPERLANE_MERKLE_TREE_KEYS, self.contract, &self.state_root);
        for idx in 0..HYPERLANE_MERKLE_TREE_KEYS.len() {
            assert_eq!(self.snapshot.branch[idx], self.branch_proof.get_branch_node(idx));
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HyperlaneMessageOutputs {
    pub state_root: String,
    // todo: output just the id, not the entire message
    pub messages: Vec<HyperlaneMessage>,
}

impl HyperlaneMessageOutputs {
    pub fn new(state_root: String, messages: Vec<HyperlaneMessage>) -> Self {
        Self { state_root, messages }
    }
}
