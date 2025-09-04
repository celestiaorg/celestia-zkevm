#[cfg(test)]
use crate::proto::celestia::prover::v1::{
    prover_client::ProverClient, prover_server::ProverServer, GetBlockProofRequest, GetBlockProofsInRangeRequest,
    GetLatestBlockProofRequest,
};
#[cfg(test)]
use crate::prover::service::ProverService;
#[cfg(test)]
use crate::storage::proof_storage::RocksDbProofStorage;
#[cfg(test)]
use celestia_types::nmt::Namespace;
#[cfg(test)]
use evm_exec_types::BlockExecOutput;
#[cfg(test)]
use sp1_sdk::{
    ProverClient as SP1ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1PublicValues, SP1_CIRCUIT_VERSION,
};
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use tempfile::TempDir;
#[cfg(test)]
use tokio::net::TcpListener;
#[cfg(test)]
use tonic::transport::{Channel, Server};

#[cfg(test)]
fn create_mock_proof() -> SP1ProofWithPublicValues {
    let (pk, _vk) = SP1ProverClient::from_env().setup(crate::prover::prover::EVM_EXEC_ELF);
    let public_values = SP1PublicValues::from(&[10, 20, 30, 40, 50]);
    SP1ProofWithPublicValues::create_mock_proof(&pk, public_values, SP1ProofMode::Plonk, SP1_CIRCUIT_VERSION)
}

#[cfg(test)]
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

#[cfg(test)]
async fn setup_test_server() -> (ProverClient<Channel>, ProverService, TempDir, String) {
    // Create test service
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    let proof_storage = Arc::new(RocksDbProofStorage::new(storage_path).unwrap());

    let prover_client = SP1ProverClient::from_env();
    let (_, vkey) = prover_client.setup(crate::prover::prover::EVM_EXEC_ELF);

    let service = ProverService::new_for_test(proof_storage.clone(), vkey);

    // Start gRPC server on available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let addr_str = format!("http://{addr}");

    let server_service = service.clone();
    tokio::spawn(async move {
        Server::builder()
            .add_service(ProverServer::new(server_service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create client
    let client = ProverClient::connect(addr_str.clone()).await.unwrap();

    (client, service, temp_dir, addr_str)
}

    #[tokio::test]
    async fn test_grpc_get_block_proof_integration() {
        let (mut client, service, _temp_dir, _addr) = setup_test_server().await;

        // Store some test data
        let proof = create_mock_proof();
        let output = create_mock_block_output();
        service
            .proof_storage()
            .store_block_proof(42, &proof, &output)
            .await
            .unwrap();

        // Test gRPC call
        let request = tonic::Request::new(GetBlockProofRequest { celestia_height: 42 });

        let response = client.get_block_proof(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.proof.is_some());
        let stored_proof = inner.proof.unwrap();
        assert_eq!(stored_proof.celestia_height, 42);
    }

    #[tokio::test]
    async fn test_grpc_get_block_proofs_in_range_integration() {
        let (mut client, service, _temp_dir, _addr) = setup_test_server().await;

        // Store multiple test proofs
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        for height in [10, 15, 20, 25, 30] {
            service
                .proof_storage()
                .store_block_proof(height, &proof, &output)
                .await
                .unwrap();
        }

        // Test gRPC call
        let request = tonic::Request::new(GetBlockProofsInRangeRequest {
            start_height: 15,
            end_height: 25,
        });

        let response = client.get_block_proofs_in_range(request).await.unwrap();
        let inner = response.into_inner();

        assert_eq!(inner.proofs.len(), 3);
        let heights: Vec<u64> = inner.proofs.iter().map(|p| p.celestia_height).collect();
        assert_eq!(heights, vec![15, 20, 25]);
    }

    #[tokio::test]
    async fn test_grpc_get_latest_block_proof_integration() {
        let (mut client, service, _temp_dir, _addr) = setup_test_server().await;

        // Store test data
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        service
            .proof_storage()
            .store_block_proof(10, &proof, &output)
            .await
            .unwrap();
        service
            .proof_storage()
            .store_block_proof(20, &proof, &output)
            .await
            .unwrap();
        service
            .proof_storage()
            .store_block_proof(15, &proof, &output)
            .await
            .unwrap();

        // Test gRPC call
        let request = tonic::Request::new(GetLatestBlockProofRequest {});

        let response = client.get_latest_block_proof(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.proof.is_some());
        let latest_proof = inner.proof.unwrap();
        assert_eq!(latest_proof.celestia_height, 20); // Should be the highest
    }

    #[tokio::test]
    async fn test_grpc_error_handling() {
        let (mut client, _service, _temp_dir, _addr) = setup_test_server().await;

        // Test not found error
        let request = tonic::Request::new(GetBlockProofRequest { celestia_height: 999 });

        let result = client.get_block_proof(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_grpc_range_validation() {
        let (mut client, _service, _temp_dir, _addr) = setup_test_server().await;

        // Test invalid range
        let request = tonic::Request::new(GetBlockProofsInRangeRequest {
            start_height: 200,
            end_height: 100,
        });

        let result = client.get_block_proofs_in_range(request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    // Note: This test would be very slow as it actually runs the BlockRangeExecProver
    // Uncomment to test the full aggregation pipeline
    /*
    #[tokio::test]
    #[ignore] // Slow test - only run manually
    async fn test_grpc_aggregate_block_proofs_integration() {
        let (mut client, service, _temp_dir, _addr) = setup_test_server().await;

        // Store some test proofs (these would need to be real proofs for aggregation)
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        service.proof_storage().store_block_proof(10, &proof, &output).await.unwrap();
        service.proof_storage().store_block_proof(11, &proof, &output).await.unwrap();

        // Test aggregation
        let request = tonic::Request::new(AggregateBlockProofsRequest {
            start_height: 10,
            end_height: 11,
        });

        let response = client.aggregate_block_proofs(request).await.unwrap();
        let inner = response.into_inner();

        assert!(!inner.proof.is_empty());
        assert!(!inner.public_values.is_empty());
    }
    */
