use prost::Name;
use serde::{Deserialize, Serialize};

/// Message for updating ZK Execution ISM (corresponds to MsgUpdateZKExecutionISM)
#[derive(Clone, Serialize, Deserialize, prost::Message)]
pub struct MsgUpdateZkExecutionIsm {
    /// ISM identifier
    #[prost(string, tag = "1")]
    pub id: String,
    /// Block height for the state transition
    #[prost(uint64, tag = "2")]
    pub height: u64,
    /// ZK proof bytes
    #[prost(bytes = "vec", tag = "3")]
    pub proof: Vec<u8>,
    /// Public values/inputs for proof verification
    #[prost(bytes = "vec", tag = "4")]
    pub public_values: Vec<u8>,
    /// The tx signer
    #[prost(string, tag = "5")]
    pub signer: String,
}

/// Response for MsgUpdateZKExecutionISM
#[derive(Clone, Serialize, Deserialize, prost::Message)]
pub struct MsgUpdateZkExecutionIsmResponse {
    /// Updated state root
    #[prost(string, tag = "1")]
    pub state_root: String,
    /// Block height
    #[prost(uint64, tag = "2")]
    pub height: u64,
}

/// Message for submitting messages with state membership proof (corresponds to MsgSubmitMessages)
#[derive(Clone, Serialize, Deserialize, prost::Message)]
pub struct MsgSubmitMessages {
    /// ISM identifier
    #[prost(string, tag = "1")]
    pub id: String,
    /// Block height for the state membership proof
    #[prost(uint64, tag = "2")]
    pub height: u64,
    /// ZK proof bytes for state membership
    #[prost(bytes = "vec", tag = "3")]
    pub proof: Vec<u8>,
    /// Public values/inputs for proof verification
    #[prost(bytes = "vec", tag = "4")]
    pub public_values: Vec<u8>,
    /// The tx signer
    #[prost(string, tag = "5")]
    pub signer: String,
}

/// Response for MsgSubmitMessages
#[derive(Clone, Serialize, Deserialize, prost::Message)]
pub struct MsgSubmitMessagesResponse {
    // Empty response according to the protobuf definition
}

// Legacy aliases for backward compatibility
pub type StateTransitionProofMsg = MsgUpdateZkExecutionIsm;
pub type StateInclusionProofMsg = MsgSubmitMessages;

impl MsgUpdateZkExecutionIsm {
    /// Create a new ZK execution ISM update message
    pub fn new(id: String, height: u64, proof: Vec<u8>, public_values: Vec<u8>, signer: String) -> Self {
        Self {
            id,
            height,
            proof,
            public_values,
            signer,
        }
    }
}

impl MsgSubmitMessages {
    /// Create a new message submission with state membership proof
    pub fn new(id: String, height: u64, proof: Vec<u8>, public_values: Vec<u8>, signer: String) -> Self {
        Self {
            id,
            height,
            proof,
            public_values,
            signer,
        }
    }
}

impl Name for MsgUpdateZkExecutionIsm {
    const NAME: &'static str = "MsgUpdateZKExecutionISM";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}

impl Name for MsgSubmitMessages {
    const NAME: &'static str = "MsgSubmitMessages";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}
