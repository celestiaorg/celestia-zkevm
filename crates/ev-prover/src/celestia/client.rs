use crate::celestia::error::{ProofSubmissionError, Result};
use crate::celestia::lumina_compat::LuminaCompatClient;
use crate::celestia::message::{StateInclusionProofMsg, StateTransitionProofMsg};
use crate::celestia::types::{ClientConfig, ProofSubmissionResponse};

use async_trait::async_trait;
use tracing::{debug, info};

/// Trait for proof submission operations
#[async_trait]
pub trait ProofSubmitter {
    /// Submit a state transition proof to Celestia
    async fn submit_state_transition_proof(
        &self,
        proof_msg: StateTransitionProofMsg,
    ) -> Result<ProofSubmissionResponse>;

    /// Submit a state inclusion proof to Celestia
    async fn submit_state_inclusion_proof(&self, proof_msg: StateInclusionProofMsg) -> Result<ProofSubmissionResponse>;
}

/// Celestia gRPC client for proof submission
#[derive(Debug)]
pub struct CelestiaProofClient {
    lumina_client: LuminaCompatClient,
}

impl CelestiaProofClient {
    /// Create a new Celestia proof client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        debug!("Creating Celestia proof client with endpoint: {}", config.grpc_endpoint);

        let lumina_client = LuminaCompatClient::new(config).await?;

        info!("Successfully created Celestia proof client");

        Ok(Self { lumina_client })
    }

    /// Create a client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = ClientConfig {
            grpc_endpoint: std::env::var("CELESTIA_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9090".to_string()),
            private_key_hex: std::env::var("CELESTIA_PRIVATE_KEY").map_err(|_| {
                ProofSubmissionError::Configuration("CELESTIA_PRIVATE_KEY environment variable not set".to_string())
            })?,
            chain_id: std::env::var("CELESTIA_CHAIN_ID").unwrap_or_else(|_| "celestia-zkevm-testnet".to_string()),
            gas_price: std::env::var("CELESTIA_GAS_PRICE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|_| ProofSubmissionError::Configuration("Invalid CELESTIA_GAS_PRICE".to_string()))?,
            max_gas: std::env::var("CELESTIA_MAX_GAS")
                .unwrap_or_else(|_| "200000".to_string())
                .parse()
                .map_err(|_| ProofSubmissionError::Configuration("Invalid CELESTIA_MAX_GAS".to_string()))?,
            confirmation_timeout: std::env::var("CELESTIA_CONFIRMATION_TIMEOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|_| {
                    ProofSubmissionError::Configuration("Invalid CELESTIA_CONFIRMATION_TIMEOUT".to_string())
                })?,
        };

        Self::new(config).await
    }

    /// Get the client configuration
    pub fn config(&self) -> &ClientConfig {
        self.lumina_client.config()
    }

    /// Get the configured chain ID
    pub fn chain_id(&self) -> &str {
        self.lumina_client.chain_id()
    }

    /// Get the configured gRPC endpoint
    pub fn grpc_endpoint(&self) -> &str {
        self.lumina_client.grpc_endpoint()
    }
}

#[async_trait]
impl ProofSubmitter for CelestiaProofClient {
    async fn submit_state_transition_proof(
        &self,
        proof_msg: StateTransitionProofMsg,
    ) -> Result<ProofSubmissionResponse> {
        self.lumina_client.submit_state_transition_proof(proof_msg).await
    }

    async fn submit_state_inclusion_proof(&self, proof_msg: StateInclusionProofMsg) -> Result<ProofSubmissionResponse> {
        self.lumina_client.submit_state_inclusion_proof(proof_msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ClientConfig {
        ClientConfig {
            grpc_endpoint: "http://localhost:9090".to_string(),
            private_key_hex: "0123456789abcdef".repeat(8), // 64 hex chars
            chain_id: "test-chain".to_string(),
            gas_price: 1000,
            max_gas: 200_000,
            confirmation_timeout: 30,
        }
    }

    #[test]
    fn test_state_transition_proof_message_structure() {
        // Test the new message structure based on actual Celestia PR #5788
        let proof_msg = StateTransitionProofMsg::new(
            "".to_string(), // Empty ISM ID should be validated
            100,            // height
            vec![1, 2, 3],  // proof
            vec![4, 5, 6],  // public_values
        );

        // Test the new field structure
        assert_eq!(proof_msg.id, "");
        assert_eq!(proof_msg.height, 100);
        assert_eq!(proof_msg.proof, vec![1, 2, 3]);
        assert_eq!(proof_msg.public_values, vec![4, 5, 6]);
    }

    #[test]
    fn test_state_inclusion_proof_message_structure() {
        // Test the new message structure based on actual Celestia PR #5790
        let proof_msg = StateInclusionProofMsg::new(
            "test-ism".to_string(), // ISM ID
            200,                    // height
            vec![7, 8, 9],          // proof
            vec![10, 11, 12],       // public_values
        );

        // Test the new field structure
        assert_eq!(proof_msg.id, "test-ism");
        assert_eq!(proof_msg.height, 200);
        assert_eq!(proof_msg.proof, vec![7, 8, 9]);
        assert_eq!(proof_msg.public_values, vec![10, 11, 12]);
    }

    #[test]
    fn test_client_config_usage() {
        let config = create_test_config();

        // Test that config fields are accessible and properly structured
        assert_eq!(config.grpc_endpoint, "http://localhost:9090");
        assert_eq!(config.chain_id, "test-chain");
        assert_eq!(config.gas_price, 1000);
        assert_eq!(config.max_gas, 200_000);
        assert_eq!(config.confirmation_timeout, 30);

        // Test that private key is properly formatted (64 hex chars)
        assert_eq!(config.private_key_hex.len(), 128); // 64 bytes * 2 chars per byte
    }
}
