#![allow(dead_code)]
use anyhow::Result;
use std::sync::Arc;
use storage::proofs::{ProofStorage, RocksDbProofStorage};
use tonic::{Request, Response, Status};

use crate::config::config::{Config, APP_HOME};
use crate::proto::celestia::prover::v1::prover_server::Prover;
use crate::proto::celestia::prover::v1::{
    BlockProof, GetBlockProofRequest, GetBlockProofResponse, GetBlockProofsInRangeRequest,
    GetBlockProofsInRangeResponse, GetLatestBlockProofRequest, GetLatestBlockProofResponse,
    GetLatestMembershipProofRequest, GetLatestMembershipProofResponse, GetMembershipProofRequest,
    GetMembershipProofResponse, GetRangeProofsRequest, GetRangeProofsResponse, MembershipProof, RangeProof,
};
use crate::prover::programs::range::BlockRangeExecProver;

pub struct ProverService {
    block_range_prover: BlockRangeExecProver,
    proof_storage: Arc<dyn ProofStorage>,
}

impl ProverService {
    pub fn new(config: Config) -> Result<Self> {
        let block_range_prover = BlockRangeExecProver::default();

        // Initialize proof storage with the path from config or default
        // Use the same default path as BlockExecProver: ~/.ev-prover/data/proofs.db
        let storage_path = config.proof_storage_path.unwrap_or_else(|| {
            dirs::home_dir()
                .expect("cannot find home directory")
                .join(APP_HOME)
                .join("data")
                .join("proofs.db")
                .to_string_lossy()
                .to_string()
        });
        let proof_storage = Arc::new(RocksDbProofStorage::new(storage_path)?);

        Ok(ProverService {
            block_range_prover,
            proof_storage,
        })
    }

    /// Creates a new ProverService with an externally provided proof storage instance.
    /// This is useful for sharing the same storage between multiple components.
    pub fn with_storage(_config: Config, proof_storage: Arc<dyn ProofStorage>) -> Result<Self> {
        let block_range_prover = BlockRangeExecProver::default();

        Ok(ProverService {
            block_range_prover,
            proof_storage,
        })
    }
}

#[tonic::async_trait]
impl Prover for ProverService {
    async fn get_block_proof(
        &self,
        request: Request<GetBlockProofRequest>,
    ) -> Result<Response<GetBlockProofResponse>, Status> {
        let req = request.into_inner();
        let celestia_height = req.celestia_height;

        let stored_proof = self
            .proof_storage
            .get_block_proof(celestia_height)
            .await
            .map_err(|e| Status::not_found(format!("Block proof not found: {e}")))?;

        let proof = BlockProof {
            celestia_height: stored_proof.celestia_height,
            proof_data: stored_proof.proof_data,
            public_values: stored_proof.public_values,
            created_at: stored_proof.created_at,
        };

        Ok(Response::new(GetBlockProofResponse { proof: Some(proof) }))
    }

    async fn get_block_proofs_in_range(
        &self,
        request: Request<GetBlockProofsInRangeRequest>,
    ) -> Result<Response<GetBlockProofsInRangeResponse>, Status> {
        let req = request.into_inner();
        let start_height = req.start_height;
        let end_height = req.end_height;

        let stored_proofs = self
            .proof_storage
            .get_block_proofs_in_range(start_height, end_height)
            .await
            .map_err(|e| Status::internal(format!("Failed to get block proofs: {e}")))?;

        let proofs: Vec<BlockProof> = stored_proofs
            .into_iter()
            .map(|p| BlockProof {
                celestia_height: p.celestia_height,
                proof_data: p.proof_data,
                public_values: p.public_values,
                created_at: p.created_at,
            })
            .collect();

        Ok(Response::new(GetBlockProofsInRangeResponse { proofs }))
    }

    async fn get_latest_block_proof(
        &self,
        _request: Request<GetLatestBlockProofRequest>,
    ) -> Result<Response<GetLatestBlockProofResponse>, Status> {
        let stored_proof_opt = self
            .proof_storage
            .get_latest_block_proof()
            .await
            .map_err(|e| Status::internal(format!("Failed to get latest block proof: {e}")))?;

        match stored_proof_opt {
            Some(stored_proof) => {
                let proof = BlockProof {
                    celestia_height: stored_proof.celestia_height,
                    proof_data: stored_proof.proof_data,
                    public_values: stored_proof.public_values,
                    created_at: stored_proof.created_at,
                };
                Ok(Response::new(GetLatestBlockProofResponse {
                    proof: Some(proof),
                    has_proof: true,
                }))
            }
            None => Ok(Response::new(GetLatestBlockProofResponse {
                proof: None,
                has_proof: false,
            })),
        }
    }

    async fn get_membership_proof(
        &self,
        request: Request<GetMembershipProofRequest>,
    ) -> Result<Response<GetMembershipProofResponse>, Status> {
        let req = request.into_inner();
        let height = req.height;

        let stored_proof = self
            .proof_storage
            .get_membership_proof(height)
            .await
            .map_err(|e| Status::not_found(format!("Membership proof not found: {e}")))?;

        let proof = MembershipProof {
            proof_data: stored_proof.proof_data,
            public_values: stored_proof.public_values,
            created_at: stored_proof.created_at,
        };

        Ok(Response::new(GetMembershipProofResponse { proof: Some(proof) }))
    }

    async fn get_latest_membership_proof(
        &self,
        _request: Request<GetLatestMembershipProofRequest>,
    ) -> Result<Response<GetLatestMembershipProofResponse>, Status> {
        let stored_proof_opt = self
            .proof_storage
            .get_latest_membership_proof()
            .await
            .map_err(|e| Status::internal(format!("Failed to get latest membership proof: {e}")))?;

        match stored_proof_opt {
            Some(stored_proof) => {
                let proof = MembershipProof {
                    proof_data: stored_proof.proof_data,
                    public_values: stored_proof.public_values,
                    created_at: stored_proof.created_at,
                };
                Ok(Response::new(GetLatestMembershipProofResponse {
                    proof: Some(proof),
                    has_proof: true,
                }))
            }
            None => Ok(Response::new(GetLatestMembershipProofResponse {
                proof: None,
                has_proof: false,
            })),
        }
    }

    async fn get_range_proofs(
        &self,
        request: Request<GetRangeProofsRequest>,
    ) -> Result<Response<GetRangeProofsResponse>, Status> {
        let req = request.into_inner();
        let start_height = req.start_height;
        let end_height = req.end_height;

        let stored_proofs = self
            .proof_storage
            .get_range_proofs(start_height, end_height)
            .await
            .map_err(|e| Status::internal(format!("Failed to get range proofs: {e}")))?;

        let proofs: Vec<RangeProof> = stored_proofs
            .into_iter()
            .map(|p| RangeProof {
                start_height: p.start_height,
                end_height: p.end_height,
                proof_data: p.proof_data,
                public_values: p.public_values,
                created_at: p.created_at,
            })
            .collect();

        Ok(Response::new(GetRangeProofsResponse { proofs }))
    }
}
