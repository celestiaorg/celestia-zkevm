use crate::{proto::celestia::zkism::v1::MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm};
use prost::Name;

// Legacy aliases for backward compatibility
pub type StateTransitionProofMsg = MsgUpdateZkExecutionIsm;
pub type StateInclusionProofMsg = MsgSubmitMessages;
pub type HyperlaneMessage = MsgProcessMessage;

/// Message for Hyperlane Warp token transfers
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgWarpTransfer {
    /// Sender address (bech32 encoded)
    #[prost(string, tag = "1")]
    pub sender: String,
    /// Token ID (32 bytes hex encoded)
    #[prost(string, tag = "2")]
    pub token_id: String,
    /// Destination domain ID
    #[prost(uint32, tag = "3")]
    pub destination_domain: u32,
    /// Recipient address (hex encoded)
    #[prost(string, tag = "4")]
    pub recipient: String,
    /// Amount to transfer (as string)
    #[prost(string, tag = "5")]
    pub amount: String,
    /// Custom hook ID (optional)
    #[prost(string, optional, tag = "6")]
    pub custom_hook_id: Option<String>,
    /// Gas limit (as string)
    #[prost(string, tag = "7")]
    pub gas_limit: String,
    /// Maximum fee
    #[prost(message, optional, tag = "8")]
    pub max_fee: Option<Coin>,
    /// Custom hook metadata
    #[prost(string, tag = "9")]
    pub custom_hook_metadata: String,
}

/// Coin message for fees
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: String,
    #[prost(string, tag = "2")]
    pub amount: String,
}

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

impl Name for MsgUpdateZkExecutionIsm {
    const NAME: &'static str = "MsgUpdateZKExecutionISM";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}

impl Name for MsgSubmitMessages {
    const NAME: &'static str = "MsgSubmitMessages";
    const PACKAGE: &'static str = "celestia.zkism.v1";
}

impl MsgWarpTransfer {
    /// Create a new warp transfer message
    pub fn new(
        sender: String,
        token_id: String,
        destination_domain: u32,
        recipient: String,
        amount: String,
        max_hyperlane_fee: String,
    ) -> Self {
        Self {
            sender,
            token_id,
            destination_domain,
            recipient,
            amount,
            custom_hook_id: None,
            gas_limit: "0".to_string(),
            max_fee: Some(Coin {
                denom: "utia".to_string(),
                amount: max_hyperlane_fee,
            }),
            custom_hook_metadata: String::new(),
        }
    }
}

impl Name for MsgWarpTransfer {
    const NAME: &'static str = "MsgRemoteTransfer";
    const PACKAGE: &'static str = "hyperlane.warp.v1";
}

impl Name for Coin {
    const NAME: &'static str = "Coin";
    const PACKAGE: &'static str = "cosmos.base.v1beta1";
}
