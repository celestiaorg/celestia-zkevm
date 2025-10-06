//! Celestia gRPC Client for x/zkism
//!
//! This crate provides a gRPC client for the x/zkism Celestia module.
//! In supports transaction submission for both state transition and state inclusion proofs
//! as well as gRPC module queries.

pub mod client;
pub mod error;
pub mod message;
pub mod proto;
pub mod types;

pub use client::{CelestiaIsmClient, ProofSubmitter};
pub use error::{IsmClientError, Result};
pub use message::{MsgWarpTransfer, StateInclusionProofMsg, StateTransitionProofMsg};
pub use proto::celestia::zkism::v1::{
    MsgSubmitMessages, MsgSubmitMessagesResponse, MsgUpdateZkExecutionIsm, MsgUpdateZkExecutionIsmResponse,
};
pub use types::ProofSubmissionResponse;
