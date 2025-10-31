//! Hyperlane message types for ISM, hooks, and mailbox operations
//!
//! These types correspond to the hyperlane-cosmos proto definitions but are
//! defined manually here to avoid complex proto generation dependencies.

use celestia_grpc::IntoProtobufAny;
use prost::Message;
use tendermint_proto::google::protobuf::Any;

// ============================================================================
// ISM Messages
// ============================================================================

/// MsgCreateNoopIsm creates a no-op ISM (for testing)
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateNoopIsm {
    #[prost(string, tag = "1")]
    pub creator: String,
}

/// Response for MsgCreateNoopIsm
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateNoopIsmResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

/// MsgCreateMerkleRootMultisigIsm creates a multisig ISM
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMerkleRootMultisigIsm {
    #[prost(string, tag = "1")]
    pub creator: String,
    #[prost(string, repeated, tag = "2")]
    pub validators: Vec<String>,
    #[prost(uint32, tag = "3")]
    pub threshold: u32,
}

/// Response for MsgCreateMerkleRootMultisigIsm
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMerkleRootMultisigIsmResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

/// MsgAnnounceValidator announces a validator for multisig ISM
#[derive(Clone, PartialEq, Message)]
pub struct MsgAnnounceValidator {
    #[prost(string, tag = "1")]
    pub validator: String,
    #[prost(string, tag = "2")]
    pub storage_location: String,
    #[prost(string, tag = "3")]
    pub signature: String,
    #[prost(string, tag = "4")]
    pub mailbox_id: String,
    #[prost(string, tag = "5")]
    pub creator: String,
}

/// Response for MsgAnnounceValidator
#[derive(Clone, PartialEq, Message)]
pub struct MsgAnnounceValidatorResponse {}

// ============================================================================
// Hook Messages
// ============================================================================

/// MsgCreateNoopHook creates a no-op hook
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateNoopHook {
    #[prost(string, tag = "1")]
    pub owner: String,
}

/// Response for MsgCreateNoopHook
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateNoopHookResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

/// MsgCreateMerkleTreeHook creates a merkle tree hook
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMerkleTreeHook {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(string, tag = "2")]
    pub mailbox_id: String,
}

/// Response for MsgCreateMerkleTreeHook
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMerkleTreeHookResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

// ============================================================================
// Mailbox Messages
// ============================================================================

/// MsgCreateMailbox creates a mailbox
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMailbox {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(uint32, tag = "2")]
    pub local_domain: u32,
    #[prost(string, tag = "3")]
    pub default_ism: String,
    #[prost(string, optional, tag = "4")]
    pub default_hook: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub required_hook: Option<String>,
}

/// Response for MsgCreateMailbox
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateMailboxResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

/// MsgSetMailbox updates mailbox settings
#[derive(Clone, PartialEq, Message)]
pub struct MsgSetMailbox {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(string, tag = "2")]
    pub mailbox_id: String,
    #[prost(string, optional, tag = "3")]
    pub default_ism: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub default_hook: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub required_hook: Option<String>,
    #[prost(bool, tag = "6")]
    pub renounce_ownership: bool,
}

/// Response for MsgSetMailbox
#[derive(Clone, PartialEq, Message)]
pub struct MsgSetMailboxResponse {}

// ============================================================================
// Warp Token Messages
// ============================================================================

/// MsgCreateCollateralToken creates a collateral warp token
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateCollateralToken {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(string, tag = "2")]
    pub origin_mailbox: String,
    #[prost(string, tag = "3")]
    pub origin_denom: String,
}

/// Response for MsgCreateCollateralToken
#[derive(Clone, PartialEq, Message)]
pub struct MsgCreateCollateralTokenResponse {
    #[prost(string, tag = "1")]
    pub id: String,
}

/// MsgSetToken updates token settings
#[derive(Clone, PartialEq, Message)]
pub struct MsgSetToken {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(string, tag = "2")]
    pub token_id: String,
    #[prost(string, optional, tag = "3")]
    pub ism_id: Option<String>,
    #[prost(string, tag = "4")]
    pub new_owner: String,
    #[prost(bool, tag = "5")]
    pub renounce_ownership: bool,
}

/// Response for MsgSetToken
#[derive(Clone, PartialEq, Message)]
pub struct MsgSetTokenResponse {}

/// RemoteRouter for warp token
#[derive(Clone, PartialEq, Message)]
pub struct RemoteRouter {
    #[prost(uint32, tag = "1")]
    pub receiver_domain: u32,
    #[prost(string, tag = "2")]
    pub receiver_contract: String,
    #[prost(string, tag = "3")]
    pub gas: String,
}

/// MsgEnrollRemoteRouter enrolls a remote router for warp token
#[derive(Clone, PartialEq, Message)]
pub struct MsgEnrollRemoteRouter {
    #[prost(string, tag = "1")]
    pub owner: String,
    #[prost(string, tag = "2")]
    pub token_id: String,
    #[prost(message, optional, tag = "3")]
    pub remote_router: Option<RemoteRouter>,
}

/// Response for MsgEnrollRemoteRouter
#[derive(Clone, PartialEq, Message)]
pub struct MsgEnrollRemoteRouterResponse {}

// ============================================================================
// Message Type URLs (for Any encoding)
// ============================================================================

pub const MSG_CREATE_NOOP_ISM_TYPE_URL: &str = "/hyperlane.core.interchain_security.v1.MsgCreateNoopIsm";
pub const MSG_CREATE_MERKLE_ROOT_MULTISIG_ISM_TYPE_URL: &str = "/hyperlane.core.interchain_security.v1.MsgCreateMerkleRootMultisigIsm";
pub const MSG_ANNOUNCE_VALIDATOR_TYPE_URL: &str = "/hyperlane.core.interchain_security.v1.MsgAnnounceValidator";
pub const MSG_CREATE_NOOP_HOOK_TYPE_URL: &str = "/hyperlane.core.post_dispatch.v1.MsgCreateNoopHook";
pub const MSG_CREATE_MERKLE_TREE_HOOK_TYPE_URL: &str = "/hyperlane.core.post_dispatch.v1.MsgCreateMerkleTreeHook";
pub const MSG_CREATE_MAILBOX_TYPE_URL: &str = "/hyperlane.core.v1.MsgCreateMailbox";
pub const MSG_SET_MAILBOX_TYPE_URL: &str = "/hyperlane.core.v1.MsgSetMailbox";
pub const MSG_CREATE_COLLATERAL_TOKEN_TYPE_URL: &str = "/hyperlane.warp.v1.MsgCreateCollateralToken";
pub const MSG_SET_TOKEN_TYPE_URL: &str = "/hyperlane.warp.v1.MsgSetToken";
pub const MSG_ENROLL_REMOTE_ROUTER_TYPE_URL: &str = "/hyperlane.warp.v1.MsgEnrollRemoteRouter";

// ============================================================================
// IntoProtobufAny implementations
// ============================================================================

impl IntoProtobufAny for MsgCreateNoopIsm {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_NOOP_ISM_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgCreateMerkleRootMultisigIsm {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_MERKLE_ROOT_MULTISIG_ISM_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgAnnounceValidator {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_ANNOUNCE_VALIDATOR_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgCreateNoopHook {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_NOOP_HOOK_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgCreateMerkleTreeHook {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_MERKLE_TREE_HOOK_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgCreateMailbox {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_MAILBOX_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgSetMailbox {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_SET_MAILBOX_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgCreateCollateralToken {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_CREATE_COLLATERAL_TOKEN_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgSetToken {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_SET_TOKEN_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}

impl IntoProtobufAny for MsgEnrollRemoteRouter {
    fn into_any(self) -> Any {
        Any {
            type_url: MSG_ENROLL_REMOTE_ROUTER_TYPE_URL.to_string(),
            value: self.encode_to_vec(),
        }
    }
}
