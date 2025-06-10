//! An SP1 program that verifies inclusion of an EVM block in a Celestia block and executes it.
//!
//! 1. Accepts an EVM block and associated Celestia proofs
//! 2. Verifies that the EVM block was included in the Celestia block
//! 3. Executes the EVM block
//! 4. Commits the resulting EVM block metadata and hash as public outputs

#![no_main]

sp1_zkvm::entrypoint!(main);

use std::sync::Arc;

use celestia_types::{blob::Blob, hash::Hash, AppVersion, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::{simple_merkle::proof::Proof, simple_merkle::tree::MerkleHash, TmSha2Hasher};
use rsp_client_executor::{executor::EthClientExecutor, io::EthClientExecutorInput};
use sha3::{Digest, Keccak256};
use tendermint::Hash as TmHash;

pub fn main() {
    // -----------------------------
    // 1. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-start: deserialize input");

    let input: KeccakInclusionToDataRootProofInput = sp1_zkvm::io::read();
    let data_root_hash = Hash::Sha256(input.data_root);

    let client_executor_input: EthClientExecutorInput =
        bincode::deserialize(&sp1_zkvm::io::read_vec())
            .expect("Failed to deserialize EVM block input");

    let celestia_header_hash: TmHash = sp1_zkvm::io::read();
    let data_hash_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
    let celestia_proof: Proof<TmSha2Hasher> = sp1_zkvm::io::read();
    let trusted_state_root: Vec<u8> = sp1_zkvm::io::read_vec();

    println!("cycle-tracker-end: deserialize input");

    // -----------------------------
    // 2. Check trusted state root
    // -----------------------------
    println!("cycle-tracker-start: assert trusted state root");

    assert_eq!(
        client_executor_input.parent_state.state_root().as_slice(),
        trusted_state_root,
        "Current block state root does not match trusted root"
    );

    println!("cycle-tracker-end: assert trusted state root");

    // -----------------------------
    // 3. Build Blob and compute hash
    // -----------------------------
    println!("cycle-tracker-start: create blob");

    let blob = Blob::new(input.namespace_id, input.data, AppVersion::V3)
        .expect("Failed to construct Celestia blob");

    println!("cycle-tracker-end: create blob");

    println!("cycle-tracker-start: compute keccak hash");

    let computed_keccak_hash: [u8; 32] =
        Keccak256::new().chain_update(&blob.data).finalize().into();

    println!("cycle-tracker-end: compute keccak hash");

    // -----------------------------
    // 4. Construct ShareProof
    // -----------------------------
    println!("cycle-tracker-start: convert blob to shares");

    let share_data = blob.to_shares().expect("Failed to convert blob to shares");

    let share_proof = ShareProof {
        data: share_data
            .into_iter()
            .map(|s| s.as_ref().try_into().expect("Invalid share length"))
            .collect(),
        namespace_id: input.namespace_id,
        share_proofs: input.share_proofs,
        row_proof: input.row_proof,
    };

    println!("cycle-tracker-end: convert blob to shares");

    // -----------------------------
    // 5. Verify data root inclusion in Celestia header
    // -----------------------------
    println!("cycle-tracker-start: verify data root");

    let hasher = TmSha2Hasher {};
    celestia_proof
        .verify_range(
            celestia_header_hash
                .as_bytes()
                .try_into()
                .expect("Invalid Celestia hash length"),
            &[hasher.hash_leaf(&data_hash_bytes)],
        )
        .expect("Celestia inclusion proof failed");

    println!("cycle-tracker-end: verify data root");

    println!("cycle-tracker-start: verify share proof");

    share_proof
        .verify(data_root_hash)
        .expect("ShareProof verification failed");

    println!("cycle-tracker-end: verify share proof");

    // -----------------------------
    // 6. Validate keccak hash matches
    // -----------------------------
    println!("cycle-tracker-start: check keccak hash");

    assert_eq!(
        computed_keccak_hash, input.keccak_hash,
        "Computed keccak hash does not match input"
    );

    println!("cycle-tracker-end: check keccak hash");

    // -----------------------------
    // 7. Execute EVM block
    // -----------------------------
    println!("cycle-tracker-start: execute EVM block");

    let executor = EthClientExecutor::eth(
        Arc::new(
            (&client_executor_input.genesis)
                .try_into()
                .expect("Invalid genesis block"),
        ),
        client_executor_input.custom_beneficiary,
    );

    let header = executor
        .execute(client_executor_input)
        .expect("EVM block execution failed");

    println!("cycle-tracker-end: execute EVM block");

    // -----------------------------
    // 8. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let output = EvmBlockExecOutput {
        blob_commitment: blob.commitment.into(),
        header_hash: header.hash_slow().into(),
        prev_header_hash: header.parent_hash.into(),
        height: header.number,
        gas_used: header.gas_used,
        beneficiary: header.beneficiary.into(),
        state_root: header.state_root.into(),
        celestia_header_hash: celestia_header_hash
            .as_bytes()
            .try_into()
            .expect("Invalid Celestia header hash length"),
        trusted_state_root: trusted_state_root
            .try_into()
            .expect("trusted state root must be exactly 32 bytes"),
    };

    sp1_zkvm::io::commit(&output);

    println!("cycle-tracker-end: commit public outputs");
}
