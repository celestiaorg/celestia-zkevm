//! RISC0-specific tests for the prover integration
//!
//! These tests verify the RISC0 integration without requiring full proof generation.
//! They test serialization, storage integration, and basic data flow.

#[cfg(all(test, feature = "risc0"))]
mod tests {
    use crate::prover::programs::block::{Risc0BlockExecInput, EV_EXEC_PROGRAM_ID};
    use alloy_primitives::FixedBytes;
    use celestia_types::{nmt::Namespace, AppVersion, DataAvailabilityHeader};
    use ev_zkevm_types::programs::block::BlockExecOutput;
    use storage::proofs::{ProofStorage, ProofStorageError, RocksDbProofStorage};
    use tempfile::TempDir;

    fn create_test_storage() -> (RocksDbProofStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbProofStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    fn create_mock_risc0_input() -> Risc0BlockExecInput {
        use celestia_types::nmt::NamespacedHash;

        let namespace = Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();

        // Create minimal valid DAH with at least 2 row_roots and 2 col_roots
        // Use Default to create empty NamespacedHash instances
        let row_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let col_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let dah = DataAvailabilityHeader::new(row_roots, col_roots, AppVersion::V3).unwrap();

        // For testing, we use an empty zeth_inputs vec
        // Real inputs would contain actual Zeth execution witnesses
        Risc0BlockExecInput {
            header_raw: vec![1, 2, 3, 4, 5],
            dah,
            blobs_raw: vec![6, 7, 8, 9, 10],
            pub_key: vec![0u8; 32],
            namespace,
            proofs: Vec::new(),
            zeth_inputs: Vec::new(), // Simplified for testing
            trusted_height: 99,
            trusted_root: FixedBytes::from([0u8; 32]),
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

    #[test]
    fn test_risc0_input_serialization() {
        let input = create_mock_risc0_input();

        // Test that we can serialize the input
        let serialized = bincode::serialize(&input).expect("Failed to serialize Risc0BlockExecInput");
        assert!(!serialized.is_empty(), "Serialized input should not be empty");

        // Test that we can deserialize it back
        let deserialized: Risc0BlockExecInput =
            bincode::deserialize(&serialized).expect("Failed to deserialize Risc0BlockExecInput");

        // Verify basic fields match
        assert_eq!(deserialized.header_raw, input.header_raw);
        assert_eq!(deserialized.blobs_raw, input.blobs_raw);
        assert_eq!(deserialized.pub_key, input.pub_key);
        assert_eq!(deserialized.namespace, input.namespace);
        assert_eq!(deserialized.trusted_height, input.trusted_height);
        assert_eq!(deserialized.trusted_root, input.trusted_root);
    }

    #[test]
    fn test_risc0_program_id_is_valid() {
        // Verify the RISC0 program ID is not empty
        assert!(!EV_EXEC_PROGRAM_ID.is_empty(), "RISC0 program ID should not be empty");

        // Verify it's 32 bytes (RISC0 ImageID size)
        assert_eq!(EV_EXEC_PROGRAM_ID.len(), 32, "RISC0 ImageID should be 32 bytes");

        // Verify it's not all zeros
        assert_ne!(EV_EXEC_PROGRAM_ID, &[0u8; 32], "RISC0 ImageID should not be all zeros");
    }

    #[tokio::test]
    async fn test_store_and_retrieve_risc0_block_proof() {
        let (storage, _temp_dir) = create_test_storage();
        let output = create_mock_block_output();

        // Create mock proof data (in reality this would come from RISC0 prover)
        let mock_proof_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mock_public_values = bincode::serialize(&output).unwrap();

        // Store the RISC0 proof
        storage
            .store_block_proof(
                42,
                storage::proofs::ProofSystem::Risc0,
                &mock_proof_data,
                &mock_public_values,
                &output,
            )
            .await
            .unwrap();

        // Retrieve the proof
        let retrieved_proof = storage.get_block_proof(42).await.unwrap();

        assert_eq!(retrieved_proof.celestia_height, 42);
        assert_eq!(retrieved_proof.proof_system, storage::proofs::ProofSystem::Risc0);
        assert_eq!(retrieved_proof.proof_data, mock_proof_data);
    }

    #[tokio::test]
    async fn test_multiple_risc0_proofs_in_range() {
        let (storage, _temp_dir) = create_test_storage();
        let output = create_mock_block_output();

        let mock_proof_data = vec![1, 2, 3, 4];
        let mock_public_values = bincode::serialize(&output).unwrap();

        // Store multiple RISC0 proofs
        for height in [10, 15, 20, 25, 30] {
            storage
                .store_block_proof(
                    height,
                    storage::proofs::ProofSystem::Risc0,
                    &mock_proof_data,
                    &mock_public_values,
                    &output,
                )
                .await
                .unwrap();
        }

        // Retrieve proofs in range
        let proofs_in_range = storage.get_block_proofs_in_range(15, 25).await.unwrap();

        assert_eq!(proofs_in_range.len(), 3);
        let heights: Vec<u64> = proofs_in_range.iter().map(|p| p.celestia_height).collect();
        assert_eq!(heights, vec![15, 20, 25]);

        // Verify all are RISC0 proofs
        for proof in proofs_in_range {
            assert_eq!(proof.proof_system, storage::proofs::ProofSystem::Risc0);
        }
    }

    #[tokio::test]
    async fn test_risc0_latest_proof() {
        let (storage, _temp_dir) = create_test_storage();
        let output = create_mock_block_output();

        let mock_proof_data = vec![1, 2, 3, 4];
        let mock_public_values = bincode::serialize(&output).unwrap();

        // Store RISC0 proofs
        storage
            .store_block_proof(
                10,
                storage::proofs::ProofSystem::Risc0,
                &mock_proof_data,
                &mock_public_values,
                &output,
            )
            .await
            .unwrap();
        storage
            .store_block_proof(
                20,
                storage::proofs::ProofSystem::Risc0,
                &mock_proof_data,
                &mock_public_values,
                &output,
            )
            .await
            .unwrap();
        storage
            .store_block_proof(
                15,
                storage::proofs::ProofSystem::Risc0,
                &mock_proof_data,
                &mock_public_values,
                &output,
            )
            .await
            .unwrap();

        // Should return the highest height proof
        let latest = storage.get_latest_block_proof().await.unwrap();
        assert!(latest.is_some());
        let latest_proof = latest.unwrap();
        assert_eq!(latest_proof.celestia_height, 20);
        assert_eq!(latest_proof.proof_system, storage::proofs::ProofSystem::Risc0);
    }

    #[test]
    fn test_risc0_input_with_empty_zeth_inputs() {
        use celestia_types::nmt::NamespacedHash;

        let namespace = Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();

        // Create minimal valid DAH with at least 2 row_roots and 2 col_roots
        // Use Default to create empty NamespacedHash instances
        let row_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let col_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let dah = DataAvailabilityHeader::new(row_roots, col_roots, AppVersion::V3).unwrap();

        let input = Risc0BlockExecInput {
            header_raw: vec![],
            dah,
            blobs_raw: vec![],
            pub_key: vec![0u8; 32],
            namespace,
            proofs: Vec::new(),
            zeth_inputs: Vec::new(), // Empty Zeth inputs
            trusted_height: 0,
            trusted_root: FixedBytes::from([0u8; 32]),
        };

        assert_eq!(input.zeth_inputs.len(), 0);

        // Should still be serializable
        let serialized = bincode::serialize(&input).expect("Should serialize even with empty zeth_inputs");
        assert!(!serialized.is_empty());
    }

    #[tokio::test]
    async fn test_risc0_proof_not_found() {
        let (storage, _temp_dir) = create_test_storage();

        // Try to retrieve a non-existent RISC0 proof
        let result = storage.get_block_proof(999).await;

        match result {
            Err(ProofStorageError::ProofNotFound(height)) => {
                assert_eq!(height, 999);
            }
            _ => panic!("Expected ProofNotFound error"),
        }
    }

    #[test]
    fn test_risc0_namespace_handling() {
        use celestia_types::nmt::NamespacedHash;

        let namespace = Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();

        // Create minimal valid DAH with at least 2 row_roots and 2 col_roots
        // Use Default to create empty NamespacedHash instances
        let row_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let col_roots = vec![NamespacedHash::default(), NamespacedHash::default()];
        let dah = DataAvailabilityHeader::new(row_roots, col_roots, AppVersion::V3).unwrap();

        let input = Risc0BlockExecInput {
            header_raw: vec![],
            dah,
            blobs_raw: vec![],
            pub_key: vec![0u8; 32],
            namespace,
            proofs: Vec::new(),
            zeth_inputs: Vec::new(),
            trusted_height: 0,
            trusted_root: FixedBytes::from([0u8; 32]),
        };

        // Verify namespace is preserved through serialization
        let serialized = bincode::serialize(&input).unwrap();
        let deserialized: Risc0BlockExecInput = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.namespace, namespace);
    }

    #[test]
    fn test_risc0_input_fields() {
        let input = create_mock_risc0_input();

        // Verify all fields are accessible and have expected types
        assert!(!input.header_raw.is_empty());
        assert!(!input.blobs_raw.is_empty());
        assert_eq!(input.pub_key.len(), 32);
        assert_eq!(input.trusted_height, 99);
        assert_eq!(input.trusted_root, FixedBytes::from([0u8; 32]));
    }

    #[test]
    fn test_block_output_serialization() {
        let output = create_mock_block_output();

        // Test serialization/deserialization of output
        let serialized = bincode::serialize(&output).unwrap();
        let deserialized: BlockExecOutput = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.new_height, output.new_height);
        assert_eq!(deserialized.prev_height, output.prev_height);
        assert_eq!(deserialized.new_state_root, output.new_state_root);
        assert_eq!(deserialized.celestia_header_hash, output.celestia_header_hash);
    }
}
