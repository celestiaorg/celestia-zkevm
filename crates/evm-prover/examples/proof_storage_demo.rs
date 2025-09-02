use celestia_types::nmt::Namespace;
use evm_exec_types::BlockExecOutput;
use evm_prover::storage::{ProofStorage, RocksDbProofStorage};
use sp1_sdk::{SP1ProofWithPublicValues, SP1PublicValues};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize storage
    let storage = RocksDbProofStorage::new("./demo_proofs.db")?;

    // Create a mock proof and output (in practice, these come from the actual prover)
    let mock_proof = SP1ProofWithPublicValues {
        proof: sp1_sdk::SP1Proof::Compressed(Box::new(sp1_sdk::SP1CompressedProof {
            bytes: vec![1, 2, 3, 4, 5],
        })),
        stdin: sp1_sdk::SP1Stdin::new(),
        public_values: SP1PublicValues::from(vec![10, 20, 30, 40, 50]),
    };

    let mock_output = BlockExecOutput {
        celestia_header_hash: [1; 32],
        prev_celestia_header_hash: [2; 32],
        new_height: 100,
        new_state_root: [3; 32],
        prev_height: 99,
        prev_state_root: [4; 32],
        namespace: Namespace::new_v0(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap(),
        public_key: [5; 32],
    };

    // Store the proof
    println!("Storing proof for Celestia height 42...");
    storage.store_block_proof(42, &mock_proof, &mock_output).await?;

    // Retrieve the proof
    println!("Retrieving proof for Celestia height 42...");
    let stored_proof = storage.get_block_proof(42).await?;

    println!("Retrieved proof:");
    println!("  Celestia Height: {}", stored_proof.celestia_height);
    println!("  EVM Height: {}", stored_proof.evm_height);
    println!("  EVM State Root: {:x?}", stored_proof.evm_state_root);
    println!("  Namespace: {:x?}", stored_proof.namespace);
    println!("  Proof Size: {} bytes", stored_proof.proof_data.len());

    // Store multiple proofs to demonstrate range queries
    for height in [10, 15, 20, 25] {
        storage.store_block_proof(height, &mock_proof, &mock_output).await?;
    }

    // Query proofs in range
    println!("\nQuerying proofs in range 12-22:");
    let proofs_in_range = storage.get_block_proofs_in_range(12, 22).await?;
    for proof in proofs_in_range {
        println!("  Found proof at height: {}", proof.celestia_height);
    }

    // Get latest proof
    let latest = storage.get_latest_block_proof().await?;
    if let Some(latest_proof) = latest {
        println!("\nLatest proof is at height: {}", latest_proof.celestia_height);
    }

    println!("\nProof storage demo completed successfully!");
    Ok(())
}
