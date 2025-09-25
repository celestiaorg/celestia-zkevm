use crate::celestia::{
    CelestiaProofClient, ClientConfig, ProofSubmitter, StateInclusionProofMsg, StateTransitionProofMsg,
};
use anyhow::{anyhow, Result};
use tracing::{debug, info};

/// Transaction client for submitting proofs to Celestia consensus network
///
/// This replaces the celestia-rpc crate for proof submission operations,
/// as celestia-rpc is designed for data availability nodes, not consensus transactions.
#[derive(Debug)]
pub struct CelestiaTxClient {
    inner: CelestiaProofClient,
}

impl CelestiaTxClient {
    /// Create a new Celestia transaction client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        let inner = CelestiaProofClient::new(config).await?;
        Ok(Self { inner })
    }

    /// Create a client from environment variables
    pub async fn from_env() -> Result<Self> {
        let inner = CelestiaProofClient::from_env().await?;
        Ok(Self { inner })
    }

    /// Submit a state transition proof to Celestia
    pub async fn submit_state_transition_proof(
        &self,
        ism_id: String,
        height: u64,
        proof: Vec<u8>,
        public_values: Vec<u8>,
    ) -> Result<String> {
        debug!(
            "Submitting state transition proof for ISM {}, height {}",
            ism_id, height
        );

        let proof_msg = StateTransitionProofMsg::new(ism_id, height, proof, public_values);

        let response = self
            .inner
            .submit_state_transition_proof(proof_msg)
            .await
            .map_err(|e| anyhow!("Failed to submit state transition proof: {}", e))?;

        info!(
            "Successfully submitted state transition proof: tx_hash={}",
            response.tx_hash
        );
        Ok(response.tx_hash)
    }

    /// Submit a state inclusion proof to Celestia
    ///
    /// This corresponds to MsgSubmitMessages from celestia-app PR #5790
    pub async fn submit_state_inclusion_proof(
        &self,
        ism_id: String,
        height: u64,
        proof: Vec<u8>,
        public_values: Vec<u8>,
    ) -> Result<String> {
        debug!("Submitting state inclusion proof for ISM {}, height {}", ism_id, height);

        let proof_msg = StateInclusionProofMsg::new(ism_id, height, proof, public_values);

        let response = self
            .inner
            .submit_state_inclusion_proof(proof_msg)
            .await
            .map_err(|e| anyhow!("Failed to submit state inclusion proof: {}", e))?;

        info!(
            "Successfully submitted state inclusion proof: tx_hash={}",
            response.tx_hash
        );
        Ok(response.tx_hash)
    }

    /// Get the configured chain ID
    pub fn chain_id(&self) -> &str {
        self.inner.chain_id()
    }

    /// Get the configured gRPC endpoint
    pub fn grpc_endpoint(&self) -> &str {
        self.inner.grpc_endpoint()
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

    #[tokio::test]
    async fn test_client_creation() {
        let config = create_test_config();

        // This will fail without a running Celestia node, but tests the basic structure
        let result = CelestiaTxClient::new(config).await;

        // In test environment without Celestia node, expect connection error
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to build") || error_msg.contains("connection"));
    }

    #[test]
    fn test_config_validation() {
        let config = create_test_config();
        assert_eq!(config.grpc_endpoint, "http://localhost:9090");
        assert_eq!(config.chain_id, "test-chain");
        assert_eq!(config.gas_price, 1000);
        assert_eq!(config.max_gas, 200_000);
        assert_eq!(config.private_key_hex.len(), 128); // 64 bytes * 2 hex chars
    }
}
