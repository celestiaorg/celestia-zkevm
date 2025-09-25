use crate::celestia::error::{ProofSubmissionError, Result};
use crate::celestia::message::{StateInclusionProofMsg, StateTransitionProofMsg};
use crate::celestia::types::{ClientConfig, ProofSubmissionResponse};

use anyhow::Context;
use celestia_grpc::GrpcClient;
use prost::Message;
use tracing::{debug, info};

/// Lumina-compatible client that handles the prost version differences
#[derive(Debug)]
pub struct LuminaCompatClient {
    grpc_client: GrpcClient,
    config: ClientConfig,
}

impl LuminaCompatClient {
    /// Create a new Lumina-compatible client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        debug!(
            "Creating Lumina-compatible client with endpoint: {}",
            config.grpc_endpoint
        );

        let grpc_client = GrpcClient::builder()
            .url(&config.grpc_endpoint)
            .private_key_hex(&config.private_key_hex)
            .build()
            .context("Failed to build Lumina gRPC client")?;

        info!("Successfully created Lumina-compatible client");

        Ok(Self { grpc_client, config })
    }

    /// Submit a state transition proof using raw protobuf bytes
    pub async fn submit_state_transition_proof(
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

        debug!("Converting MsgUpdateZkExecutionIsm for submission to Celestia");

        // Create transaction config
        let tx_config = celestia_grpc::TxConfig {
            gas_limit: Some(self.config.max_gas),
            gas_price: Some(self.config.gas_price as f64),
            memo: Some("zkISM state transition proof submission".to_string()),
            ..Default::default()
        };

        // Create protobuf bytes and submit via Lumina
        let mut buf = Vec::new();
        proof_msg.encode(&mut buf).context("Failed to encode proof message")?;

        // Create tendermint_proto::google::protobuf::Any directly
        let any_msg = tendermint_proto::google::protobuf::Any {
            type_url: "/celestia.zkism.v1.MsgUpdateZkExecutionIsm".to_string(),
            value: buf,
        };

        match self.grpc_client.submit_message(any_msg, tx_config).await {
            Ok(tx_info) => {
                info!(
                    "Successfully submitted state transition proof: tx_hash={}, height={}",
                    tx_info.hash,
                    tx_info.height.value()
                );

                Ok(ProofSubmissionResponse {
                    tx_hash: tx_info.hash.to_string(),
                    height: tx_info.height.value(),
                    gas_used: 0, // TxInfo doesn't provide gas_used
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => Err(ProofSubmissionError::SubmissionFailed(format!(
                "Failed to submit state transition proof: {e}"
            ))),
        }
    }

    /// Submit a state inclusion proof
    pub async fn submit_state_inclusion_proof(
        &self,
        proof_msg: StateInclusionProofMsg,
    ) -> Result<ProofSubmissionResponse> {
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

        debug!("Converting MsgSubmitMessages for submission to Celestia");

        // Create transaction config
        let tx_config = celestia_grpc::TxConfig {
            gas_limit: Some(self.config.max_gas),
            gas_price: Some(self.config.gas_price as f64),
            memo: Some("zkISM state inclusion proof submission".to_string()),
            ..Default::default()
        };

        // Create protobuf bytes and submit via Lumina
        let mut buf = Vec::new();
        proof_msg.encode(&mut buf).context("Failed to encode proof message")?;

        // Create tendermint_proto::google::protobuf::Any directly
        let any_msg = tendermint_proto::google::protobuf::Any {
            type_url: "/celestia.zkism.v1.MsgSubmitMessages".to_string(),
            value: buf,
        };

        match self.grpc_client.submit_message(any_msg, tx_config).await {
            Ok(tx_info) => {
                info!(
                    "Successfully submitted state inclusion proof: tx_hash={}, height={}",
                    tx_info.hash,
                    tx_info.height.value()
                );

                Ok(ProofSubmissionResponse {
                    tx_hash: tx_info.hash.to_string(),
                    height: tx_info.height.value(),
                    gas_used: 0, // TxInfo doesn't provide gas_used
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => Err(ProofSubmissionError::SubmissionFailed(format!(
                "Failed to submit state inclusion proof: {e}"
            ))),
        }
    }

    /// Get the configured chain ID
    pub fn chain_id(&self) -> &str {
        &self.config.chain_id
    }

    /// Get the configured gRPC endpoint
    pub fn grpc_endpoint(&self) -> &str {
        &self.config.grpc_endpoint
    }

    /// Get the client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}
