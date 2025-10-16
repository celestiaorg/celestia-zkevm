#![allow(dead_code)]
use anyhow::Result;
use std::sync::Arc;
use storage::proofs::ProofStorage;
use tonic::{Request, Response, Status};

use crate::config::config::Config;
use crate::proto::celestia::prover::v1::prover_server::Prover;
use crate::proto::celestia::prover::v1::{
    BlockProof, GetBlockProofRequest, GetBlockProofResponse, GetBlockProofsInRangeRequest,
    GetBlockProofsInRangeResponse, GetLatestBlockProofRequest, GetLatestBlockProofResponse,
    GetLatestMembershipProofRequest, GetLatestMembershipProofResponse, GetMembershipProofRequest,
    GetMembershipProofResponse, GetRangeProofsRequest, GetRangeProofsResponse, MembershipProof, RangeProof,
};

pub struct ProverService {
    proof_storage: Arc<dyn ProofStorage>,
}

impl ProverService {
    /// Creates a new ProverService with an externally provided proof storage instance.
    /// This is useful for sharing the same storage between multiple components.
    pub fn with_storage(_config: Config, proof_storage: Arc<dyn ProofStorage>) -> Result<Self> {
        Ok(ProverService { proof_storage })
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
        let stored_proof = self
            .proof_storage
            .get_latest_block_proof()
            .await
            .map_err(|e| Status::internal(format!("Failed to get latest block proof: {e}")))?
            .ok_or_else(|| Status::not_found("No block proofs found in storage"))?;

        let proof = BlockProof {
            celestia_height: stored_proof.celestia_height,
            proof_data: stored_proof.proof_data,
            public_values: stored_proof.public_values,
            created_at: stored_proof.created_at,
        };

        Ok(Response::new(GetLatestBlockProofResponse { proof: Some(proof) }))
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
        let stored_proof = self
            .proof_storage
            .get_latest_membership_proof()
            .await
            .map_err(|e| Status::internal(format!("Failed to get latest membership proof: {e}")))?
            .ok_or_else(|| Status::not_found("No membership proofs found in storage"))?;

        let proof = MembershipProof {
            proof_data: stored_proof.proof_data,
            public_values: stored_proof.public_values,
            created_at: stored_proof.created_at,
        };

        Ok(Response::new(GetLatestMembershipProofResponse { proof: Some(proof) }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use storage::proofs::testing::MockProofStorage;
    use storage::proofs::{StoredBlockProof, StoredMembershipProof, StoredRangeProof};

    fn create_test_service() -> (ProverService, Arc<MockProofStorage>) {
        let mock_storage = Arc::new(MockProofStorage::new());
        let service = ProverService {
            proof_storage: mock_storage.clone(),
        };
        (service, mock_storage)
    }

    #[tokio::test]
    async fn test_get_block_proof_success() {
        let (service, mock) = create_test_service();

        let stored_proof = StoredBlockProof {
            celestia_height: 100,
            proof_data: vec![1, 2, 3, 4],
            public_values: vec![5, 6, 7, 8],
            created_at: 1234567890,
        };
        mock.insert_block_proof(100, stored_proof);

        let request = Request::new(GetBlockProofRequest { celestia_height: 100 });
        let response = service.get_block_proof(request).await.unwrap();
        let proof = response.into_inner().proof.unwrap();

        assert_eq!(proof.celestia_height, 100);
        assert_eq!(proof.proof_data, vec![1, 2, 3, 4]);
        assert_eq!(proof.public_values, vec![5, 6, 7, 8]);
    }

    #[tokio::test]
    async fn test_get_block_proof_not_found() {
        let (service, _mock) = create_test_service();

        let request = Request::new(GetBlockProofRequest { celestia_height: 999 });
        let result = service.get_block_proof(request).await;

        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_get_block_proofs_in_range() {
        let (service, mock) = create_test_service();

        for height in [30, 31, 32, 33, 34, 35] {
            let stored_proof = StoredBlockProof {
                celestia_height: height,
                proof_data: vec![1, 2, 3, 4],
                public_values: vec![5, 6, 7, 8],
                created_at: 1234567890,
            };
            mock.insert_block_proof(height, stored_proof);
        }

        let request = Request::new(GetBlockProofsInRangeRequest {
            start_height: 31,
            end_height: 34,
        });
        let response = service.get_block_proofs_in_range(request).await.unwrap();
        let proofs = response.into_inner().proofs;

        assert_eq!(proofs.len(), 4);
        assert_eq!(proofs[0].celestia_height, 31);
        assert_eq!(proofs[3].celestia_height, 34);
    }

    #[tokio::test]
    async fn test_get_latest_block_proof_success() {
        let (service, mock) = create_test_service();

        for height in [30, 31, 32] {
            let stored_proof = StoredBlockProof {
                celestia_height: height,
                proof_data: vec![1, 2, 3, 4],
                public_values: vec![height as u8],
                created_at: 1234567890,
            };
            mock.insert_block_proof(height, stored_proof);
        }

        let request = Request::new(GetLatestBlockProofRequest {});
        let response = service.get_latest_block_proof(request).await.unwrap();
        let proof = response.into_inner().proof.unwrap();

        assert_eq!(proof.celestia_height, 32);
        assert_eq!(proof.public_values, vec![32]);
    }

    #[tokio::test]
    async fn test_get_latest_block_proof_empty_storage() {
        let (service, _mock) = create_test_service();

        let request = Request::new(GetLatestBlockProofRequest {});
        let result = service.get_latest_block_proof(request).await;

        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("No block proofs found"));
    }

    #[tokio::test]
    async fn test_get_membership_proof_success() {
        let (service, mock) = create_test_service();

        let stored_proof = StoredMembershipProof {
            proof_data: vec![9, 10, 11, 12],
            public_values: vec![13, 14, 15, 16],
            created_at: 1234567890,
        };
        mock.insert_membership_proof(100, stored_proof);

        let request = Request::new(GetMembershipProofRequest { height: 100 });
        let response = service.get_membership_proof(request).await.unwrap();
        let proof = response.into_inner().proof.unwrap();

        assert_eq!(proof.proof_data, vec![9, 10, 11, 12]);
        assert_eq!(proof.public_values, vec![13, 14, 15, 16]);
    }

    #[tokio::test]
    async fn test_get_membership_proof_not_found() {
        let (service, _mock) = create_test_service();

        let request = Request::new(GetMembershipProofRequest { height: 999 });
        let result = service.get_membership_proof(request).await;

        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_get_latest_membership_proof_success() {
        let (service, mock) = create_test_service();

        for height in [100, 101, 102] {
            let stored_proof = StoredMembershipProof {
                proof_data: vec![height as u8],
                public_values: vec![13, 14, 15, 16],
                created_at: 1234567890,
            };
            mock.insert_membership_proof(height, stored_proof);
        }

        let request = Request::new(GetLatestMembershipProofRequest {});
        let response = service.get_latest_membership_proof(request).await.unwrap();
        let proof = response.into_inner().proof.unwrap();

        assert_eq!(proof.proof_data, vec![102]);
    }

    #[tokio::test]
    async fn test_get_latest_membership_proof_empty_storage() {
        let (service, _mock) = create_test_service();

        let request = Request::new(GetLatestMembershipProofRequest {});
        let result = service.get_latest_membership_proof(request).await;

        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("No membership proofs found"));
    }

    #[tokio::test]
    async fn test_get_range_proofs() {
        let (service, mock) = create_test_service();

        let ranges = vec![(30, 35), (36, 40), (41, 45)];
        for (start, end) in ranges {
            let stored_proof = StoredRangeProof {
                start_height: start,
                end_height: end,
                proof_data: vec![1, 2, 3, 4],
                public_values: vec![5, 6, 7, 8],
                created_at: 1234567890,
            };
            mock.insert_range_proof(stored_proof);
        }

        let request = Request::new(GetRangeProofsRequest {
            start_height: 30,
            end_height: 45,
        });
        let response = service.get_range_proofs(request).await.unwrap();
        let proofs = response.into_inner().proofs;

        assert_eq!(proofs.len(), 3);
        assert_eq!(proofs[0].start_height, 30);
        assert_eq!(proofs[0].end_height, 35);
    }

    #[tokio::test]
    async fn test_empty_range_query() {
        let (service, _mock) = create_test_service();

        let request = Request::new(GetBlockProofsInRangeRequest {
            start_height: 100,
            end_height: 200,
        });
        let response = service.get_block_proofs_in_range(request).await.unwrap();
        let proofs = response.into_inner().proofs;

        assert_eq!(proofs.len(), 0);
    }
}
