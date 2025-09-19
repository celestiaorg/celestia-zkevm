use thiserror::Error;

/// Result type for proof submission operations
pub type Result<T> = std::result::Result<T, ProofSubmissionError>;

/// Error types for proof submission to Celestia
#[derive(Error, Debug)]
pub enum ProofSubmissionError {
    #[error("Failed to connect to Celestia validator: {0}")]
    Connection(String),

    #[error("Transaction submission failed: {0}")]
    TransactionSubmission(String),

    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Client configuration error: {0}")]
    Configuration(String),

    #[error("Timeout waiting for transaction confirmation")]
    Timeout,

    #[error("Insufficient gas or fees")]
    InsufficientGas,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Lumina gRPC error: {0}")]
    LuminaGrpc(#[from] anyhow::Error),

    #[error("Tendermint error: {0}")]
    Tendermint(String),

    #[cfg(feature = "cosmrs-support")]
    #[error("CosmRS error: {0}")]
    CosmRs(#[from] cosmrs::Error),
}