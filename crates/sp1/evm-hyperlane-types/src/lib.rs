use alloy_primitives::Address;
use evm_state_types::hyperlane::HyperlaneMessage;
use evm_storage_proofs::types::HyperlaneBranchProof;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HyperlaneMessageInputs {
    pub state_root: String,
    pub contract: Address,
    pub messages: Vec<HyperlaneMessage>,
    pub branch_proof: HyperlaneBranchProof,
}

impl HyperlaneMessageInputs {
    pub fn new(
        state_root: String,
        contract: Address,
        messages: Vec<HyperlaneMessage>,
        branch_proof: HyperlaneBranchProof,
    ) -> Self {
        Self {
            state_root,
            contract,
            messages,
            branch_proof,
        }
    }
    pub fn verify(&self) -> bool {
        let message_ids: Vec<String> = self.messages.iter().map(|m| m.id()).collect();
        false
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
