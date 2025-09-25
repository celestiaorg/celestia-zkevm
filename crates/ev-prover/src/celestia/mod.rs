pub mod client;
pub mod error;
pub mod lumina_compat;
pub mod message;
pub mod types;

pub use client::{CelestiaProofClient, ProofSubmitter};
pub use error::{ProofSubmissionError, Result};
pub use message::{
    MsgSubmitMessages, MsgSubmitMessagesResponse, MsgUpdateZkExecutionIsm, MsgUpdateZkExecutionIsmResponse,
    StateInclusionProofMsg, StateTransitionProofMsg,
};
pub use types::{ClientConfig, ProofSubmissionResponse};