use crate::{MsgSubmitMessages, MsgUpdateZkExecutionIsm};
use prost::Name;

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
