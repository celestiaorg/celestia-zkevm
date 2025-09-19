use serde::{Deserialize, Serialize};

/// Message for updating ZK Execution ISM (corresponds to MsgUpdateZKExecutionISM)
/// From celestia-app PR #5788: proto/celestia/zkism/v1/tx.proto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgUpdateZkExecutionIsm {
    /// ISM identifier
    pub id: String,
    /// Block height for the state transition
    pub height: u64,
    /// ZK proof bytes
    pub proof: Vec<u8>,
    /// Public values/inputs for proof verification
    pub public_values: Vec<u8>,
}

/// Response for MsgUpdateZKExecutionISM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgUpdateZkExecutionIsmResponse {
    /// Updated state root
    pub state_root: String,
    /// Block height
    pub height: u64,
}

/// Message for submitting messages with state membership proof (corresponds to MsgSubmitMessages)
/// From celestia-app PR #5790: proto/celestia/zkism/v1/tx.proto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgSubmitMessages {
    /// ISM identifier
    pub id: String,
    /// Block height for the state membership proof
    pub height: u64,
    /// ZK proof bytes for state membership
    pub proof: Vec<u8>,
    /// Public values/inputs for proof verification
    pub public_values: Vec<u8>,
}

/// Response for MsgSubmitMessages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgSubmitMessagesResponse {
    // Empty response according to the protobuf definition
}

// Legacy aliases for backward compatibility
pub type StateTransitionProofMsg = MsgUpdateZkExecutionIsm;
pub type StateInclusionProofMsg = MsgSubmitMessages;

impl MsgUpdateZkExecutionIsm {
    /// Create a new ZK execution ISM update message
    pub fn new(
        id: String,
        height: u64,
        proof: Vec<u8>,
        public_values: Vec<u8>,
    ) -> Self {
        Self {
            id,
            height,
            proof,
            public_values,
        }
    }
}

impl MsgSubmitMessages {
    /// Create a new message submission with state membership proof
    pub fn new(
        id: String,
        height: u64,
        proof: Vec<u8>,
        public_values: Vec<u8>,
    ) -> Self {
        Self {
            id,
            height,
            proof,
            public_values,
        }
    }
}

