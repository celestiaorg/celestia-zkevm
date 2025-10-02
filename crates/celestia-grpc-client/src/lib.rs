//! Celestia gRPC Client for Proof Submission
//!
//! This crate provides a gRPC client for submitting state transition and state inclusion
//! proofs to the Celestia consensus network. It reuses the Lumina gRPC library for
//! underlying communication with Celestia validator nodes.

pub mod client;
pub mod error;
pub mod message;
pub mod proto;
pub mod types;

pub use client::{CelestiaIsmClient, ProofSubmitter};
pub use error::{IsmClientError, Result};
pub use message::{
    MsgSubmitMessages, MsgSubmitMessagesResponse, MsgUpdateZkExecutionIsm, MsgUpdateZkExecutionIsmResponse,
    StateInclusionProofMsg, StateTransitionProofMsg,
};
pub use types::ProofSubmissionResponse;
