use crate::error::{ProofSubmissionError, Result};
use crate::message::{StateInclusionProofMsg, StateTransitionProofMsg};
use crate::types::{ClientConfig, ProofSubmissionResponse};

use anyhow::Context;
use async_trait::async_trait;
use celestia_grpc::GrpcClient;
use tracing::{debug, info, warn};

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
pub struct CelestiaProofClient {
    grpc_client: GrpcClient,
    config: ClientConfig,
}

impl CelestiaProofClient {
    /// Create a new Celestia proof client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        debug!("Creating Celestia proof client with endpoint: {}", config.grpc_endpoint);

        let grpc_client = GrpcClient::builder()
            .url(&config.grpc_endpoint)
            .private_key_hex(&config.private_key_hex)
            .build()
            .context("Failed to build Lumina gRPC client")?;

        info!("Successfully created Celestia proof client");

        Ok(Self { grpc_client, config })
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

    /// Get the gRPC client reference for direct access to Lumina functionality
    pub fn grpc_client(&self) -> &GrpcClient {
        &self.grpc_client
    }

    /// Get the client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get the configured chain ID
    pub fn chain_id(&self) -> &str {
        &self.config.chain_id
    }

    /// Get the configured gRPC endpoint
    pub fn grpc_endpoint(&self) -> &str {
        &self.config.grpc_endpoint
    }

    /// Submit a zkISM proof message via Lumina
    async fn submit_zkism_message<M>(&self, message: M, message_type: &str) -> Result<ProofSubmissionResponse>
    where
        M: celestia_grpc::IntoProtobufAny + Send + 'static,
    {
        debug!(
            "Submitting {} message to Celestia via Lumina (endpoint: {}, chain: {})",
            message_type, self.config.grpc_endpoint, self.config.chain_id
        );

        // Create transaction config
        let tx_config = celestia_grpc::TxConfig {
            gas_limit: Some(self.config.max_gas),
            gas_price: Some(self.config.gas_price as f64),
            memo: Some(format!("zkISM {message_type} submission")),
            ..Default::default()
        };

        // Submit via Lumina
        match self.grpc_client.submit_message(message, tx_config).await {
            Ok(tx_info) => {
                info!(
                    "Successfully submitted {} message: tx_hash={}, height={}",
                    message_type,
                    tx_info.hash,
                    tx_info.height.value()
                );

                Ok(ProofSubmissionResponse {
                    tx_hash: tx_info.hash.to_string(),
                    height: tx_info.height.value(),
                    gas_used: 0, // TxInfo doesn't provide gas_used, use estimation
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                warn!("Failed to submit {} message: {}", message_type, e);
                Err(ProofSubmissionError::SubmissionFailed(format!(
                    "Failed to submit {message_type}: {e}"
                )))
            }
        }
    }
}

#[async_trait]
impl ProofSubmitter for CelestiaProofClient {
    async fn submit_state_transition_proof(
        &self,
        proof_msg: StateTransitionProofMsg,
    ) -> Result<ProofSubmissionResponse> {
        info!(
            "Submitting state transition proof for ISM id: {}, height: {}",
            proof_msg.id, proof_msg.height
        );

        // Validate proof message
        if proof_msg.proof.is_empty() {
            return Err(ProofSubmissionError::InvalidProof(
                "Proof data cannot be empty".to_string(),
            ));
        }

        if proof_msg.id.is_empty() {
            return Err(ProofSubmissionError::InvalidProof("ISM ID cannot be empty".to_string()));
        }

        // Submit via Lumina
        self.submit_zkism_message(proof_msg, "MsgUpdateZKExecutionISM").await
    }

    async fn submit_state_inclusion_proof(&self, proof_msg: StateInclusionProofMsg) -> Result<ProofSubmissionResponse> {
        info!(
            "Submitting state inclusion proof for ISM id: {}, height: {}",
            proof_msg.id, proof_msg.height
        );

        // Validate proof message
        if proof_msg.proof.is_empty() {
            return Err(ProofSubmissionError::InvalidProof(
                "Proof data cannot be empty".to_string(),
            ));
        }

        if proof_msg.id.is_empty() {
            return Err(ProofSubmissionError::InvalidProof("ISM ID cannot be empty".to_string()));
        }

        // Submit via Lumina
        self.submit_zkism_message(proof_msg, "MsgSubmitMessages").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
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
            "".to_string(),            // Empty ISM ID should be validated
            100,                       // height
            vec![1, 2, 3],             // proof
            vec![4, 5, 6],             // public_values
            "test_signer".to_string(), // signer
        );

        // Test the new field structure
        assert_eq!(proof_msg.id, "");
        assert_eq!(proof_msg.height, 100);
        assert_eq!(proof_msg.proof, vec![1, 2, 3]);
        assert_eq!(proof_msg.public_values, vec![4, 5, 6]);
        assert_eq!(proof_msg.signer, "test_signer");
    }

    #[test]
    fn test_state_inclusion_proof_message_structure() {
        // Test the new message structure based on actual Celestia PR #5790
        let proof_msg = StateInclusionProofMsg::new(
            "test-ism".to_string(),    // ISM ID
            200,                       // height
            vec![7, 8, 9],             // proof
            vec![10, 11, 12],          // public_values
            "test_signer".to_string(), // signer
        );

        // Test the new field structure
        assert_eq!(proof_msg.id, "test-ism");
        assert_eq!(proof_msg.height, 200);
        assert_eq!(proof_msg.proof, vec![7, 8, 9]);
        assert_eq!(proof_msg.public_values, vec![10, 11, 12]);
        assert_eq!(proof_msg.signer, "test_signer");
    }

    #[test]
    fn test_message_serialization() {
        let proof_msg = StateTransitionProofMsg::new(
            "test-ism-123".to_string(),
            1000,
            vec![0xff, 0xee, 0xdd],
            vec![0x01, 0x02, 0x03],
            "test_signer".to_string(),
        );

        // Test that the message can be serialized (this validates the structure)
        let serialized = serde_json::to_vec(&proof_msg).expect("Should serialize");
        assert!(!serialized.is_empty());

        // Test deserialization
        let deserialized: StateTransitionProofMsg = serde_json::from_slice(&serialized).expect("Should deserialize");
        assert_eq!(deserialized.id, proof_msg.id);
        assert_eq!(deserialized.height, proof_msg.height);
        assert_eq!(deserialized.proof, proof_msg.proof);
        assert_eq!(deserialized.public_values, proof_msg.public_values);
        assert_eq!(deserialized.signer, proof_msg.signer);
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
