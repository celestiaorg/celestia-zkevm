#[cfg(test)]
mod tests {
    use super::*;
    use celestia_types::nmt::Namespace;
    use evm_exec_types::{BlockExecOutput, BlockRangeExecOutput};
    use sp1_sdk::{SP1ProofWithPublicValues, SP1PublicValues};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_storage() -> (RocksDbProofStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbProofStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    fn create_mock_proof() -> SP1ProofWithPublicValues {
        SP1ProofWithPublicValues {
            proof: sp1_sdk::SP1Proof::Compressed(Box::new(sp1_sdk::SP1CompressedProof {
                bytes: vec![1, 2, 3, 4, 5],
            })),
            stdin: sp1_sdk::SP1Stdin::new(),
            public_values: SP1PublicValues::from(vec![10, 20, 30, 40, 50]),
        }
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

    fn create_mock_range_output() -> BlockRangeExecOutput {
        BlockRangeExecOutput {
            celestia_header_hash: [6; 32],
            trusted_height: 90,
            trusted_state_root: [7; 32],
            new_height: 100,
            new_state_root: [8; 32],
            namespace: Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap(),
            public_key: [9; 32],
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_block_proof() {
        let (storage, _temp_dir) = create_test_storage();
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Store the proof
        storage.store_block_proof(42, &proof, &output).await.unwrap();

        // Retrieve the proof
        let retrieved_proof = storage.get_block_proof(42).await.unwrap();

        assert_eq!(retrieved_proof.celestia_height, 42);
        assert_eq!(retrieved_proof.celestia_header_hash, output.celestia_header_hash);
        assert_eq!(retrieved_proof.evm_height, output.new_height);
        assert_eq!(retrieved_proof.evm_state_root, output.new_state_root);
        assert_eq!(retrieved_proof.namespace, output.namespace);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_range_proof() {
        let (storage, _temp_dir) = create_test_storage();
        let proof = create_mock_proof();
        let output = create_mock_range_output();

        // Store the range proof
        storage.store_range_proof(10, 20, &proof, &output).await.unwrap();

        // Retrieve range proofs
        let retrieved_proofs = storage.get_range_proofs(5, 25).await.unwrap();

        assert_eq!(retrieved_proofs.len(), 1);
        let retrieved_proof = &retrieved_proofs[0];
        assert_eq!(retrieved_proof.start_height, 10);
        assert_eq!(retrieved_proof.end_height, 20);
        assert_eq!(retrieved_proof.celestia_header_hash, output.celestia_header_hash);
        assert_eq!(retrieved_proof.trusted_height, output.trusted_height);
        assert_eq!(retrieved_proof.new_height, output.new_height);
    }

    #[tokio::test]
    async fn test_get_block_proofs_in_range() {
        let (storage, _temp_dir) = create_test_storage();
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Store multiple block proofs
        for height in [10, 15, 20, 25, 30] {
            storage.store_block_proof(height, &proof, &output).await.unwrap();
        }

        // Retrieve proofs in range
        let proofs_in_range = storage.get_block_proofs_in_range(15, 25).await.unwrap();

        assert_eq!(proofs_in_range.len(), 3);
        let heights: Vec<u64> = proofs_in_range.iter().map(|p| p.celestia_height).collect();
        assert_eq!(heights, vec![15, 20, 25]);
    }

    #[tokio::test]
    async fn test_get_latest_block_proof() {
        let (storage, _temp_dir) = create_test_storage();
        let proof = create_mock_proof();
        let output = create_mock_block_output();

        // Initially should return None
        let latest = storage.get_latest_block_proof().await.unwrap();
        assert!(latest.is_none());

        // Store some proofs
        storage.store_block_proof(10, &proof, &output).await.unwrap();
        storage.store_block_proof(20, &proof, &output).await.unwrap();
        storage.store_block_proof(15, &proof, &output).await.unwrap();

        // Should return the highest height proof
        let latest = storage.get_latest_block_proof().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().celestia_height, 20);
    }


    #[tokio::test]
    async fn test_proof_not_found() {
        let (storage, _temp_dir) = create_test_storage();

        // Try to retrieve a non-existent proof
        let result = storage.get_block_proof(999).await;

        match result {
            Err(ProofStorageError::ProofNotFound(height)) => {
                assert_eq!(height, 999);
            }
            _ => panic!("Expected ProofNotFound error"),
        }
    }
}