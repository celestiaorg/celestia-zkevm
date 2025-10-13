use thiserror::Error;

/// Result type for proof submission operations
pub type Result<T> = std::result::Result<T, IsmClientError>;

/// Error types for proof submission to Celestia
#[derive(Error, Debug)]
pub enum IsmClientError {
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Client configuration error: {0}")]
    Configuration(String),

    #[error("Tx submission failed: {0}")]
    TxFailed(String),

    #[error("Lumina gRPC error: {0}")]
    LuminaGrpc(#[from] anyhow::Error),
}
