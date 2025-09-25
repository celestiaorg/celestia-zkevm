#![allow(dead_code)]
use anyhow::Result;
use tonic::{Request, Response, Status};

use crate::celestia_client::CelestiaTxClient;
use crate::config::config::Config;
use crate::proto::celestia::prover::v1::prover_server::Prover;
use crate::proto::celestia::prover::v1::{
    InfoRequest, InfoResponse, ProveStateMembershipRequest, ProveStateMembershipResponse, ProveStateTransitionRequest,
    ProveStateTransitionResponse,
};
use crate::prover::programs::range::BlockRangeExecProver;

pub struct ProverService {
    block_range_prover: BlockRangeExecProver,
    celestia_client: Option<CelestiaTxClient>,
}

impl ProverService {
    pub fn new(_config: Config) -> Result<Self> {
        let block_range_prover = BlockRangeExecProver::default();

        // Initialize Celestia client if configured via environment variables
        let celestia_client = None; // Will be set via set_celestia_client method

        Ok(ProverService {
            block_range_prover,
            celestia_client,
        })
    }

    /// Set the Celestia transaction client
    pub fn set_celestia_client(&mut self, client: CelestiaTxClient) {
        self.celestia_client = Some(client);
    }

    /// Check if Celestia client is configured
    pub fn has_celestia_client(&self) -> bool {
        self.celestia_client.is_some()
    }
}

#[tonic::async_trait]
impl Prover for ProverService {
    async fn info(&self, _request: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        let response = InfoResponse {
            state_membership_verifier_key: "".to_string(),
            state_transition_verifier_key: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn prove_state_transition(
        &self,
        request: Request<ProveStateTransitionRequest>,
    ) -> Result<Response<ProveStateTransitionResponse>, Status> {
        let req = request.into_inner();

        // TODO: Generate actual proof using the block range prover
        // For now, create a placeholder proof
        let proof_data = vec![0u8; 32]; // Placeholder proof
        let public_values = vec![0u8; 32]; // Placeholder public values

        // Submit proof to Celestia if client is configured
        if let Some(ref celestia_client) = self.celestia_client {
            match celestia_client
                .submit_state_transition_proof(
                    req.client_id.clone(),
                    0, // TODO: extract height from client_id or proof
                    proof_data.clone(),
                    public_values.clone(),
                )
                .await
            {
                Ok(tx_hash) => {
                    tracing::info!("Successfully submitted state transition proof to Celestia: {}", tx_hash);
                }
                Err(e) => {
                    tracing::warn!("Failed to submit proof to Celestia: {}", e);
                    // Don't fail the request if Celestia submission fails
                }
            }
        }

        let response = ProveStateTransitionResponse {
            proof: proof_data,
            public_values,
        };

        Ok(Response::new(response))
    }

    async fn prove_state_membership(
        &self,
        request: Request<ProveStateMembershipRequest>,
    ) -> Result<Response<ProveStateMembershipResponse>, Status> {
        let req = request.into_inner();

        // TODO: Generate actual state membership proof
        // For now, create a placeholder proof
        let proof_data = vec![0u8; 32]; // Placeholder proof
        let public_values = vec![0u8; 32]; // Placeholder public values

        // Submit proof to Celestia if client is configured
        if let Some(ref celestia_client) = self.celestia_client {
            match celestia_client
                .submit_state_inclusion_proof(
                    req.client_id.clone(),
                    0, // TODO: extract height from client_id or proof
                    proof_data.clone(),
                    public_values.clone(),
                )
                .await
            {
                Ok(tx_hash) => {
                    tracing::info!("Successfully submitted state inclusion proof to Celestia: {}", tx_hash);
                }
                Err(e) => {
                    tracing::warn!("Failed to submit inclusion proof to Celestia: {}", e);
                    // Don't fail the request if Celestia submission fails
                }
            }
        }

        let response = ProveStateMembershipResponse {
            proof: proof_data,
            height: 0, // TODO: set actual height
        };

        Ok(Response::new(response))
    }
}
