use crate::{
    proto::{celestia::zkism::v1::MsgCreateZkExecutionIsm, hyperlane::warp::v1::MsgSetToken},
    MsgProcessMessage, MsgRemoteTransfer, MsgSubmitMessages, MsgUpdateZkExecutionIsm,
};
use prost::Name;

// Legacy aliases for backward compatibility
pub type StateTransitionProofMsg = MsgUpdateZkExecutionIsm;
pub type StateInclusionProofMsg = MsgSubmitMessages;
pub type HyperlaneMessage = MsgProcessMessage;

impl Name for MsgSetToken {
    const NAME: &'static str = "MsgSetToken";
    const PACKAGE: &'static str = "hyperlane.warp.v1";
}

impl Name for MsgCreateZkExecutionIsm {
    const NAME: &'static str = "MsgCreateZKExecutionISM";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}

impl MsgUpdateZkExecutionIsm {
    /// Create a new ZK execution ISM update message
    pub fn new(id: String, proof: Vec<u8>, public_values: Vec<u8>, signer: String) -> Self {
        Self {
            id,
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

impl MsgRemoteTransfer {
    pub fn new(sender: String, token_id: String, destination_domain: u32, recipient: String, amount: String) -> Self {
        use crate::proto::cosmos::base::v1beta1::Coin;

        Self {
            sender,
            token_id,
            destination_domain,
            recipient,
            amount,
            custom_hook_id: String::new(),
            gas_limit: "0".to_string(),
            max_fee: Some(Coin {
                denom: "utia".to_string(),
                amount: "100".to_string(),
            }),
            custom_hook_metadata: String::new(),
        }
    }
}

impl Name for MsgRemoteTransfer {
    const NAME: &'static str = "MsgRemoteTransfer";
    const PACKAGE: &'static str = "hyperlane.warp.v1";
}
