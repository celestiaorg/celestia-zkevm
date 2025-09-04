#![allow(dead_code)]
use anyhow::Result;
use tonic::{Request, Response, Status};

use std::sync::Arc;

use crate::config::config::Config;
use crate::proto::celestia::prover::v1::prover_server::Prover;
use crate::proto::celestia::prover::v1::{
    AggregateBlockProofsRequest, AggregateBlockProofsResponse, GetBlockProofRequest, GetBlockProofResponse,
    GetBlockProofsInRangeRequest, GetBlockProofsInRangeResponse, GetLatestBlockProofRequest,
    GetLatestBlockProofResponse, GetRangeProofRequest, GetRangeProofResponse, InfoRequest, InfoResponse,
    ProveStateMembershipRequest, ProveStateMembershipResponse, ProveStateTransitionRequest,
    ProveStateTransitionResponse, StoredBlockProof as ProtoStoredBlockProof, StoredRangeProof as ProtoStoredRangeProof,
};
use crate::prover::prover::{BlockRangeExecProver, ProofInput};
use crate::prover::ProgramProver;
use crate::storage::proof_storage::{StoredBlockProof, StoredRangeProof};
use crate::storage::{ProofStorage, RocksDbProofStorage};
use evm_exec_types::BlockRangeExecInput;
use sp1_sdk::{HashableKey, ProverClient, SP1Proof, SP1VerifyingKey};

pub struct ProverService {
    block_range_prover: BlockRangeExecProver,
    proof_storage: Arc<dyn ProofStorage>,
    vkey: SP1VerifyingKey,
}

impl Clone for ProverService {
    fn clone(&self) -> Self {
        Self {
            block_range_prover: BlockRangeExecProver::new(), // Create new instance
            proof_storage: Arc::clone(&self.proof_storage),
            vkey: self.vkey.clone(),
        }
    }
}

impl ProverService {
    pub fn new(_config: Config) -> Result<Self> {
        let block_range_prover = BlockRangeExecProver::new();

        // Initialize proof storage
        let storage_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
            .join(".celestia-zkevm")
            .join("data")
            .join("proofs.db");

        let proof_storage = Arc::new(RocksDbProofStorage::new(storage_path)?);

        // Initialize verifying key once during service creation
        let prover_client = ProverClient::from_env();
        let (_, vkey) = prover_client.setup(crate::prover::prover::EVM_EXEC_ELF);

        Ok(ProverService {
            block_range_prover,
            proof_storage,
            vkey,
        })
    }

    /// Get a reference to the proof storage
    pub fn proof_storage(&self) -> &Arc<dyn ProofStorage> {
        &self.proof_storage
    }

    /// Parse client_id to extract block range
    /// Expected format: "start_height-end_height" (e.g., "100-200")
    fn parse_block_range(&self, client_id: &str) -> Result<(u64, u64), Box<Status>> {
        Self::parse_block_range_impl(client_id)
    }

    fn parse_block_range_impl(client_id: &str) -> Result<(u64, u64), Box<Status>> {
        let parts: Vec<&str> = client_id.split('-').collect();
        if parts.len() != 2 {
            return Err(Box::new(Status::invalid_argument(
                "client_id must be in format 'start_height-end_height'",
            )));
        }

        let start_height = parts[0]
            .parse::<u64>()
            .map_err(|_| Box::new(Status::invalid_argument("Invalid start height")))?;
        let end_height = parts[1]
            .parse::<u64>()
            .map_err(|_| Box::new(Status::invalid_argument("Invalid end height")))?;

        if start_height > end_height {
            return Err(Box::new(Status::invalid_argument("Start height must be <= end height")));
        }

        // Validate reasonable range size to prevent resource exhaustion
        const MAX_RANGE_SIZE: u64 = 1000;
        if end_height - start_height + 1 > MAX_RANGE_SIZE {
            return Err(Box::new(Status::invalid_argument(format!(
                "Range too large. Maximum allowed: {MAX_RANGE_SIZE} blocks"
            ))));
        }

        Ok((start_height, end_height))
    }

    /// Convert StoredBlockProof to proto StoredBlockProof
    fn to_proto_block_proof(stored: StoredBlockProof) -> ProtoStoredBlockProof {
        ProtoStoredBlockProof {
            celestia_height: stored.celestia_height,
            proof_data: stored.proof_data,
            public_values: stored.public_values,
            created_at: stored.created_at,
        }
    }

    /// Convert StoredRangeProof to proto StoredRangeProof
    fn to_proto_range_proof(stored: StoredRangeProof) -> ProtoStoredRangeProof {
        ProtoStoredRangeProof {
            start_height: stored.start_height,
            end_height: stored.end_height,
            proof_data: stored.proof_data,
            public_values: stored.public_values,
            created_at: stored.created_at,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(proof_storage: Arc<dyn ProofStorage>, vkey: SP1VerifyingKey) -> Self {
        Self {
            block_range_prover: BlockRangeExecProver::new(),
            proof_storage,
            vkey,
        }
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
        _request: Request<ProveStateTransitionRequest>,
    ) -> Result<Response<ProveStateTransitionResponse>, Status> {
        // TODO: Implement actual state transition proving based on Celestia light client
        Err(Status::unimplemented("State transition proving not yet implemented"))
    }

    async fn prove_state_membership(
        &self,
        _request: Request<ProveStateMembershipRequest>,
    ) -> Result<Response<ProveStateMembershipResponse>, Status> {
        Err(Status::unimplemented("prove_state_membership is unimplemented"))
    }

    async fn get_block_proof(
        &self,
        request: Request<GetBlockProofRequest>,
    ) -> Result<Response<GetBlockProofResponse>, Status> {
        let req = request.into_inner();

        let stored_proof = self
            .proof_storage
            .get_block_proof(req.celestia_height)
            .await
            .map_err(|e| Status::not_found(format!("Block proof not found: {e}")))?;

        let response = GetBlockProofResponse {
            proof: Some(Self::to_proto_block_proof(stored_proof)),
        };

        Ok(Response::new(response))
    }

    async fn get_block_proofs_in_range(
        &self,
        request: Request<GetBlockProofsInRangeRequest>,
    ) -> Result<Response<GetBlockProofsInRangeResponse>, Status> {
        let req = request.into_inner();

        // Validate range size
        const MAX_RANGE_SIZE: u64 = 1000;
        if req.end_height < req.start_height {
            return Err(Status::invalid_argument("End height must be >= start height"));
        }
        if req.end_height - req.start_height + 1 > MAX_RANGE_SIZE {
            return Err(Status::invalid_argument(format!(
                "Range too large. Maximum allowed: {MAX_RANGE_SIZE} blocks"
            )));
        }

        let stored_proofs = self
            .proof_storage
            .get_block_proofs_in_range(req.start_height, req.end_height)
            .await
            .map_err(|e| Status::internal(format!("Failed to query proof store: {e}")))?;

        let proto_proofs = stored_proofs.into_iter().map(Self::to_proto_block_proof).collect();

        let response = GetBlockProofsInRangeResponse { proofs: proto_proofs };

        Ok(Response::new(response))
    }

    async fn get_range_proof(
        &self,
        request: Request<GetRangeProofRequest>,
    ) -> Result<Response<GetRangeProofResponse>, Status> {
        let req = request.into_inner();

        let stored_proofs = self
            .proof_storage
            .get_range_proofs(req.start_height, req.end_height)
            .await
            .map_err(|e| Status::internal(format!("Failed to query range proofs: {e}")))?;

        let proto_proofs = stored_proofs.into_iter().map(Self::to_proto_range_proof).collect();

        let response = GetRangeProofResponse { proofs: proto_proofs };

        Ok(Response::new(response))
    }

    async fn get_latest_block_proof(
        &self,
        _request: Request<GetLatestBlockProofRequest>,
    ) -> Result<Response<GetLatestBlockProofResponse>, Status> {
        let stored_proof = self
            .proof_storage
            .get_latest_block_proof()
            .await
            .map_err(|e| Status::internal(format!("Failed to get latest proof: {e}")))?;

        match stored_proof {
            Some(proof) => {
                let response = GetLatestBlockProofResponse {
                    proof: Some(Self::to_proto_block_proof(proof)),
                };
                Ok(Response::new(response))
            }
            None => Err(Status::not_found("No block proofs found")),
        }
    }

    async fn aggregate_block_proofs(
        &self,
        request: Request<AggregateBlockProofsRequest>,
    ) -> Result<Response<AggregateBlockProofsResponse>, Status> {
        let req = request.into_inner();

        // Validate range size
        const MAX_RANGE_SIZE: u64 = 1000;
        if req.end_height < req.start_height {
            return Err(Status::invalid_argument("End height must be >= start height"));
        }
        if req.end_height - req.start_height + 1 > MAX_RANGE_SIZE {
            return Err(Status::invalid_argument(format!(
                "Range too large. Maximum allowed: {MAX_RANGE_SIZE} blocks"
            )));
        }

        // Query the proof store for block proofs in the requested range
        let block_proofs = self
            .proof_storage
            .get_block_proofs_in_range(req.start_height, req.end_height)
            .await
            .map_err(|e| Status::internal(format!("Failed to query proof store: {e}")))?;

        if block_proofs.is_empty() {
            return Err(Status::not_found(format!(
                "No proofs found for range {}-{}",
                req.start_height, req.end_height
            )));
        }

        // Pre-allocate vectors with known capacity for better performance
        let mut proof_inputs = Vec::with_capacity(block_proofs.len());
        let mut public_values = Vec::with_capacity(block_proofs.len());
        let vkey_hash = self.vkey.hash_u32();

        // Single iteration to build both proof_inputs and public_values
        for stored_proof in block_proofs {
            // Deserialize the SP1 proof
            let proof = SP1Proof::Compressed(
                bincode::deserialize(&stored_proof.proof_data)
                    .map_err(|e| Status::internal(format!("Failed to deserialize proof: {e}")))?,
            );

            proof_inputs.push(ProofInput {
                proof,
                vkey: self.vkey.clone(),
            });
            public_values.push(stored_proof.public_values);
        }

        // Prepare the BlockRangeExecInput with only the required fields
        let range_input = BlockRangeExecInput {
            vkeys: vec![vkey_hash; proof_inputs.len()],
            public_values,
        };

        // Generate the aggregated proof using BlockRangeExecProver
        let (aggregated_proof, _output) = self
            .block_range_prover
            .prove((range_input, proof_inputs))
            .await
            .map_err(|e| Status::internal(format!("Failed to generate aggregated proof: {e}")))?;

        let response = AggregateBlockProofsResponse {
            proof: aggregated_proof.bytes(),
            public_values: aggregated_proof.public_values.to_vec(),
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::proof_storage::RocksDbProofStorage;
    use celestia_types::nmt::Namespace;
    use evm_exec_types::BlockExecOutput;
    use sp1_sdk::{ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1PublicValues, SP1_CIRCUIT_VERSION};
    use tempfile::TempDir;
    use tonic::{Code, Request};

    // Helper functions
    fn create_mock_proof() -> SP1ProofWithPublicValues {
        let (pk, _vk) = ProverClient::from_env().setup(crate::prover::prover::EVM_EXEC_ELF);
        let public_values = SP1PublicValues::from(&[10, 20, 30, 40, 50]);
        SP1ProofWithPublicValues::create_mock_proof(&pk, public_values, SP1ProofMode::Plonk, SP1_CIRCUIT_VERSION)
    }

    fn create_mock_block_output() -> BlockExecOutput {
        BlockExecOutput {
            celestia_header_hash: [1; 32],
            prev_celestia_header_hash: [2; 32],
            new_height: 100,
            new_state_root: [3; 32],
            prev_height: 99,
            prev_state_root: [4; 32],
            namespace: Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap(),
            public_key: [5; 32],
        }
    }

    async fn create_test_service() -> (ProverService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();
        let proof_storage = Arc::new(RocksDbProofStorage::new(storage_path).unwrap());

        // Create mock verifying key for tests
        let prover_client = ProverClient::from_env();
        let (_, vkey) = prover_client.setup(crate::prover::prover::EVM_EXEC_ELF);

        let service = ProverService::new_for_test(proof_storage, vkey);

        (service, temp_dir)
    }

    #[test]
    fn test_parse_block_range_valid() {
        let (start, end) = ProverService::parse_block_range_impl("100-200").unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 200);
    }

    #[test]
    fn test_parse_block_range_invalid_format() {
        let result = ProverService::parse_block_range_impl("100");
        assert!(result.is_err());

        let result = ProverService::parse_block_range_impl("100-200-300");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_block_range_invalid_numbers() {
        let result = ProverService::parse_block_range_impl("abc-200");
        assert!(result.is_err());

        let result = ProverService::parse_block_range_impl("100-def");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_block_range_invalid_order() {
        let result = ProverService::parse_block_range_impl("200-100");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_block_range_too_large() {
        let result = ProverService::parse_block_range_impl("100-1200");
        assert!(result.is_err());

        // Exactly at limit should work
        let result = ProverService::parse_block_range_impl("100-1099");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_block_proof_success() {
        let (service, _temp_dir) = create_test_service().await;
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Store a block proof first
        service
            .proof_storage
            .store_block_proof(42, &proof, &output)
            .await
            .unwrap();

        // Test the gRPC method
        let request = Request::new(GetBlockProofRequest { celestia_height: 42 });

        let response = service.get_block_proof(request).await.unwrap();
        let inner_response = response.into_inner();

        assert!(inner_response.proof.is_some());
        let stored_proof = inner_response.proof.unwrap();
        assert_eq!(stored_proof.celestia_height, 42);
    }

    #[tokio::test]
    async fn test_get_block_proof_not_found() {
        let (service, _temp_dir) = create_test_service().await;

        let request = Request::new(GetBlockProofRequest { celestia_height: 999 });

        let result = service.get_block_proof(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn test_get_block_proofs_in_range_success() {
        let (service, _temp_dir) = create_test_service().await;
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Store multiple block proofs
        for height in [10, 15, 20, 25, 30] {
            service
                .proof_storage
                .store_block_proof(height, &proof, &output)
                .await
                .unwrap();
        }

        // Test the gRPC method
        let request = Request::new(GetBlockProofsInRangeRequest {
            start_height: 15,
            end_height: 25,
        });

        let response = service.get_block_proofs_in_range(request).await.unwrap();
        let inner_response = response.into_inner();

        assert_eq!(inner_response.proofs.len(), 3);
        let heights: Vec<u64> = inner_response.proofs.iter().map(|p| p.celestia_height).collect();
        assert_eq!(heights, vec![15, 20, 25]);
    }

    #[tokio::test]
    async fn test_get_block_proofs_in_range_invalid_range() {
        let (service, _temp_dir) = create_test_service().await;

        // Test invalid range (end < start)
        let request = Request::new(GetBlockProofsInRangeRequest {
            start_height: 200,
            end_height: 100,
        });

        let result = service.get_block_proofs_in_range(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_get_block_proofs_in_range_too_large() {
        let (service, _temp_dir) = create_test_service().await;

        // Test range too large (> 1000 blocks)
        let request = Request::new(GetBlockProofsInRangeRequest {
            start_height: 100,
            end_height: 1200,
        });

        let result = service.get_block_proofs_in_range(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
        assert!(status.message().contains("Range too large"));
    }

    #[tokio::test]
    async fn test_get_latest_block_proof_success() {
        let (service, _temp_dir) = create_test_service().await;
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Store some proofs
        service
            .proof_storage
            .store_block_proof(10, &proof, &output)
            .await
            .unwrap();
        service
            .proof_storage
            .store_block_proof(20, &proof, &output)
            .await
            .unwrap();
        service
            .proof_storage
            .store_block_proof(15, &proof, &output)
            .await
            .unwrap();

        // Test the gRPC method
        let request = Request::new(GetLatestBlockProofRequest {});

        let response = service.get_latest_block_proof(request).await.unwrap();
        let inner_response = response.into_inner();

        assert!(inner_response.proof.is_some());
        let latest_proof = inner_response.proof.unwrap();
        assert_eq!(latest_proof.celestia_height, 20); // Should be the highest
    }

    #[tokio::test]
    async fn test_get_latest_block_proof_empty() {
        let (service, _temp_dir) = create_test_service().await;

        let request = Request::new(GetLatestBlockProofRequest {});

        let result = service.get_latest_block_proof(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
        assert!(status.message().contains("No block proofs found"));
    }

    #[tokio::test]
    async fn test_proto_conversion() {
        let stored_block = StoredBlockProof {
            celestia_height: 42,
            proof_data: vec![1, 2, 3, 4],
            public_values: vec![5, 6, 7, 8],
            created_at: 1234567890,
        };

        let proto_block = ProverService::to_proto_block_proof(stored_block.clone());

        assert_eq!(proto_block.celestia_height, stored_block.celestia_height);
        assert_eq!(proto_block.proof_data, stored_block.proof_data);
        assert_eq!(proto_block.public_values, stored_block.public_values);
        assert_eq!(proto_block.created_at, stored_block.created_at);
    }
}
