use thiserror::Error;

/// Result type for proof submission operations
pub type Result<T> = std::result::Result<T, ProofSubmissionError>;

/// Error types for proof submission to Celestia
#[derive(Error, Debug)]
pub enum ProofSubmissionError {
    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Client configuration error: {0}")]
    Configuration(String),

    #[error("Proof submission failed: {0}")]
    SubmissionFailed(String),

    #[error("Lumina gRPC error: {0}")]
    LuminaGrpc(#[from] anyhow::Error),
}
