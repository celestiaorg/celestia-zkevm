use crate::error::{IsmClientError, Result};
use crate::message::{HyperlaneMessage, StateInclusionProofMsg, StateTransitionProofMsg};
use crate::proto::celestia::zkism::v1::{
    query_client::QueryClient, QueryIsmRequest, QueryIsmResponse, QueryIsmsRequest, QueryIsmsResponse,
};
use crate::types::{ClientConfig, TxResponse};

use anyhow::Context;
use async_trait::async_trait;
use celestia_grpc::GrpcClient;
use tonic::{
    transport::{Channel, Endpoint},
    Request,
};
use tracing::{debug, info, warn};

/// Trait for proof submission operations
#[async_trait]
pub trait ProofSubmitter {
    /// Submit a state transition proof to Celestia
    async fn submit_state_transition_proof(&self, proof_msg: StateTransitionProofMsg) -> Result<TxResponse>;

    /// Submit a state inclusion proof to Celestia
    async fn submit_state_inclusion_proof(&self, proof_msg: StateInclusionProofMsg) -> Result<TxResponse>;

    /// Process a Hyperlane message
    async fn process_hyperlane_message(&self, message: HyperlaneMessage) -> Result<TxResponse>;
}

/// Celestia gRPC client for proof submission
pub struct CelestiaIsmClient {
    config: ClientConfig,
    channel: Channel,
    tx_client: GrpcClient,
}

impl CelestiaIsmClient {
    /// Create a new Celestia proof client
    pub async fn new(mut config: ClientConfig) -> Result<Self> {
        debug!("Creating Celestia proof client with endpoint: {}", config.grpc_endpoint);

        // Derive and cache the signer address if not already set
        if config.signer_address.is_empty() {
            config.signer_address = ClientConfig::derive_signer_address(&config.private_key_hex)?;
            debug!("Derived signer address: {}", config.signer_address);
        }

        // optional: set timeouts, concurrency limits, TLS, etc.
        let endpoint = Endpoint::from_shared(config.grpc_endpoint.clone())?
            .connect_timeout(std::time::Duration::from_secs(15))
            .tcp_nodelay(true);

        let channel = endpoint.connect().await?;

        let tx_client = GrpcClient::builder()
            .url(&config.grpc_endpoint)
            .private_key_hex(&config.private_key_hex)
            .build()
            .context("Failed to build Lumina gRPC client")?;

        info!("Successfully created Celestia proof client");

        Ok(Self {
            config,
            channel,
            tx_client,
        })
    }

    /// Create a client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = ClientConfig {
            grpc_endpoint: std::env::var("CELESTIA_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9090".to_string()),
            private_key_hex: std::env::var("CELESTIA_PRIVATE_KEY").map_err(|_| {
                IsmClientError::Configuration("CELESTIA_PRIVATE_KEY environment variable not set".to_string())
            })?,
            signer_address: String::new(), // Will be derived in new()
            chain_id: std::env::var("CELESTIA_CHAIN_ID").unwrap_or_else(|_| "celestia-zkevm-testnet".to_string()),
            gas_price: std::env::var("CELESTIA_GAS_PRICE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|_| IsmClientError::Configuration("Invalid CELESTIA_GAS_PRICE".to_string()))?,
            max_gas: std::env::var("CELESTIA_MAX_GAS")
                .unwrap_or_else(|_| "200000".to_string())
                .parse()
                .map_err(|_| IsmClientError::Configuration("Invalid CELESTIA_MAX_GAS".to_string()))?,
            confirmation_timeout: std::env::var("CELESTIA_CONFIRMATION_TIMEOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|_| IsmClientError::Configuration("Invalid CELESTIA_CONFIRMATION_TIMEOUT".to_string()))?,
        };

        Self::new(config).await
    }

    /// Get the gRPC tx client reference for direct access to Lumina functionality
    pub fn tx_client(&self) -> &GrpcClient {
        &self.tx_client
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

    /// Get the cached bech32-encoded signer address
    pub fn signer_address(&self) -> &str {
        &self.config.signer_address
    }

    pub async fn ism(&self, req: QueryIsmRequest) -> Result<QueryIsmResponse> {
        let mut client = QueryClient::new(self.channel.clone());
        let resp = client.ism(Request::new(req)).await?;
        Ok(resp.into_inner())
    }

    pub async fn isms(&self, req: QueryIsmsRequest) -> Result<QueryIsmsResponse> {
        let mut client = QueryClient::new(self.channel.clone());
        let resp = client.isms(Request::new(req)).await?;
        Ok(resp.into_inner())
    }

    /// Sign and send a tx to Celestia including the provided message.
    async fn send_tx<M>(&self, message: M, message_type: &str) -> Result<TxResponse>
    where
        M: celestia_grpc::IntoProtobufAny + Send + 'static,
    {
        debug!(
            "Submitting {} message to Celestia via Lumina (endpoint: {}, chain: {})",
            message_type, self.config.grpc_endpoint, self.config.chain_id
        );

        let tx_config = celestia_grpc::TxConfig {
            gas_limit: Some(self.config.max_gas),
            gas_price: Some(self.config.gas_price as f64),
            memo: Some(format!("zkISM {message_type} submission")),
            ..Default::default()
        };

        match self.tx_client.submit_message(message, tx_config).await {
            Ok(tx_info) => {
                info!(
                    "Successfully submitted {} message: tx_hash={}, height={}",
                    message_type,
                    tx_info.hash,
                    tx_info.height.value()
                );

                Ok(TxResponse {
                    tx_hash: tx_info.hash.to_string(),
                    height: tx_info.height.value(),
                    gas_used: 0, // TxInfo doesn't provide gas_used, use estimation
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                warn!("Failed to submit {} message: {}", message_type, e);
                Err(IsmClientError::SubmissionFailed(format!(
                    "Failed to submit {message_type}: {e}"
                )))
            }
        }
    }
}

#[async_trait]
impl ProofSubmitter for CelestiaIsmClient {
    async fn submit_state_transition_proof(&self, proof_msg: StateTransitionProofMsg) -> Result<TxResponse> {
        info!(
            "Submitting state transition proof for ISM id: {}, height: {}",
            proof_msg.id, proof_msg.height
        );

        if proof_msg.proof.is_empty() {
            return Err(IsmClientError::InvalidProof("Proof data cannot be empty".to_string()));
        }

        if proof_msg.id.is_empty() {
            return Err(IsmClientError::InvalidProof("ISM ID cannot be empty".to_string()));
        }

        self.send_tx(proof_msg, "MsgUpdateZKExecutionISM").await
    }

    async fn submit_state_inclusion_proof(&self, proof_msg: StateInclusionProofMsg) -> Result<TxResponse> {
        info!(
            "Submitting state inclusion proof for ISM id: {}, height: {}",
            proof_msg.id, proof_msg.height
        );

        if proof_msg.proof.is_empty() {
            return Err(IsmClientError::InvalidProof("Proof data cannot be empty".to_string()));
        }

        if proof_msg.id.is_empty() {
            return Err(IsmClientError::InvalidProof("ISM ID cannot be empty".to_string()));
        }

        self.send_tx(proof_msg, "MsgSubmitMessages").await
    }

    async fn process_hyperlane_message(&self, message: HyperlaneMessage) -> Result<TxResponse> {
        info!("Processing Hyperlane message for ISM id: {}", message.mailbox_id);

        self.send_tx(message, "MsgProcessMessage").await
    }
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;

    #[allow(dead_code)]
    fn create_test_config() -> ClientConfig {
        ClientConfig {
            grpc_endpoint: "http://localhost:9090".to_string(),
            private_key_hex: "0123456789abcdef".repeat(8), // 64 hex chars
            signer_address: String::new(),                 // Will be derived
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
        let serialized = proof_msg.encode_to_vec();
        assert!(!serialized.is_empty());

        // Test deserialization
        let deserialized: StateTransitionProofMsg =
            StateTransitionProofMsg::decode(serialized.as_slice()).expect("failed to decode");

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
