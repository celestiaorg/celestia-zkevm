use serde::{Deserialize, Serialize};

/// Type of proof being submitted
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    /// State transition proof for ZK execution ISM
    StateTransition,
    /// State inclusion proof for message submission
    StateInclusion,
}

/// Response from proof submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSubmissionResponse {
    /// Transaction hash
    pub tx_hash: String,
    /// Block height where transaction was included
    pub height: u64,
    /// Gas used for the transaction
    pub gas_used: u64,
    /// Whether the transaction was successful
    pub success: bool,
    /// Error message if transaction failed
    pub error_message: Option<String>,
}

/// Configuration for the Celestia proof client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Celestia validator gRPC endpoint
    pub grpc_endpoint: String,
    /// Private key for signing transactions (hex encoded)
    pub private_key_hex: String,
    /// Chain ID for the Celestia network
    pub chain_id: String,
    /// Gas price for transactions
    pub gas_price: u64,
    /// Maximum gas limit per transaction
    pub max_gas: u64,
    /// Timeout for transaction confirmation (in seconds)
    pub confirmation_timeout: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            grpc_endpoint: "http://localhost:9090".to_string(),
            private_key_hex: String::new(),
            chain_id: "celestia-zkevm-testnet".to_string(),
            gas_price: 1000,
            max_gas: 200_000,
            confirmation_timeout: 60,
        }
    }
}