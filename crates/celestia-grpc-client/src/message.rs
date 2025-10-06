use crate::{MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm};
use prost::Name;

// Legacy aliases for backward compatibility
pub type StateTransitionProofMsg = MsgUpdateZkExecutionIsm;
pub type StateInclusionProofMsg = MsgSubmitMessages;
pub type HyperlaneMessage = MsgProcessMessage;

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

impl Name for MsgUpdateZkExecutionIsm {
    const NAME: &'static str = "MsgUpdateZKExecutionISM";
    const PACKAGE: &'static str = "celestia.zkism.v1";
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

impl Name for MsgSubmitMessages {
    const NAME: &'static str = "MsgSubmitMessages";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}

impl MsgProcessMessage {
    pub fn new(mailbox_id: String, relayer: String, metadata: String, message: String) -> Self {
        Self {
            mailbox_id,
            relayer,
            metadata,
            message,
        }
    }
}

impl Name for MsgProcessMessage {
    const NAME: &'static str = "MsgProcessMessage";
    const PACKAGE: &'static str = "hyperlane.core.v1";
}
