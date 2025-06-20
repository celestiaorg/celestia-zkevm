//! An SP1 program that verifies inclusion of an EVM reth block in the Celestia data availability network
//! and executes its state transition function.
//!
//! 1. Accepts an EVM block STF and associated Celestia proofs.
//! 2. Verifies that the EVM block was included in the Celestia block.
//! 3. Executes the EVM block STF.
//! 4. Commits the resulting EVM and Celestia block metadata as public outputs.

#![no_main]

sp1_zkvm::entrypoint!(main);

use std::sync::Arc;

use celestia_types::{blob::Blob, hash::Hash, AppVersion, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::{simple_merkle::proof::Proof, simple_merkle::tree::MerkleHash, TmSha2Hasher};
use rsp_client_executor::{executor::EthClientExecutor, io::EthClientExecutorInput};
use sha3::{Digest, Keccak256};
use tendermint::block::Header;

pub fn main() {
    // -----------------------------
    // 1. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-start: deserialize input");

    let blob_proof: KeccakInclusionToDataRootProofInput = sp1_zkvm::io::read();

    let client_executor_input: EthClientExecutorInput = sp1_zkvm::io::read();

    let celestia_header_raw: Vec<u8> = sp1_zkvm::io::read_vec();
    let celestia_header: Header =
        serde_cbor::from_slice(&celestia_header_raw).expect("failed to deserialize celestia header");

    // TODO(removal): why do we need? we can take data root from blob proof and prove that against celestia header
    let data_hash_bytes: Vec<u8> = sp1_zkvm::io::read_vec();

    let data_hash_proof: Proof<TmSha2Hasher> = sp1_zkvm::io::read();
    // TODO(removal): we probably don't need to take this as input, only commit as output from client_executor_input.parent_state.state_root
    // we can commit it as output and then plug in the trusted field from on-chain ISM data
    let trusted_state_root: Vec<u8> = sp1_zkvm::io::read_vec();

    println!("cycle-tracker-end: deserialize input");

    // -----------------------------
    // 2. Check trusted state root
    // -----------------------------
    println!("cycle-tracker-start: assert trusted state root");
    // TODO(removal): same as above. commit trusted state root as output from client_executor_input.parent_header().state_root
    // then plug in the trusted field from on-chain data when verifying proof
    assert_eq!(
        client_executor_input.parent_header().state_root.to_vec(),
        trusted_state_root,
        "parent state root does not match trusted root"
    );

    println!("cycle-tracker-end: assert trusted state root");

    // -----------------------------
    // 3. Build Blob, compute and validate hash
    // -----------------------------
    println!("cycle-tracker-start: create blob");

    let blob =
        Blob::new(blob_proof.namespace_id, blob_proof.data, AppVersion::V3).expect("failed to construct Celestia blob");

    println!("cycle-tracker-end: create blob");

    println!("cycle-tracker-start: compute keccak hash");

    let computed_keccak_hash: [u8; 32] = Keccak256::new().chain_update(&blob.data).finalize().into();

    println!("cycle-tracker-end: compute keccak hash");

    println!("cycle-tracker-start: check keccak hash");

    assert_eq!(
        computed_keccak_hash, blob_proof.keccak_hash,
        "computed keccak hash does not match input"
    );

    println!("cycle-tracker-end: check keccak hash");

    // -----------------------------
    // 4. Construct and verify ShareProof
    // -----------------------------
    println!("cycle-tracker-start: construct blob share proof");

    let data_root = Hash::Sha256(blob_proof.data_root);
    let share_data = blob.to_shares().expect("failed to convert blob to shares");

    let share_proof = ShareProof {
        data: share_data
            .into_iter()
            .map(|s| s.as_ref().try_into().expect("invalid share length"))
            .collect(),
        namespace_id: blob_proof.namespace_id,
        share_proofs: blob_proof.share_proofs,
        row_proof: blob_proof.row_proof,
    };

    println!("cycle-tracker-end: construct blob share proof");

    println!("cycle-tracker-start: verify share proof");

    share_proof.verify(data_root).expect("ShareProof verification failed");

    println!("cycle-tracker-end: verify share proof");

    // -----------------------------
    // 5. Verify data root inclusion in Celestia header
    // -----------------------------
    println!("cycle-tracker-start: verify data root");

    let hasher = TmSha2Hasher {};
    data_hash_proof
        .verify_range(
            celestia_header.hash().as_bytes().try_into().unwrap(),
            &[hasher.hash_leaf(&data_hash_bytes)],
        )
        .expect("Celestia inclusion proof failed");

    println!("cycle-tracker-end: verify data root");

    // -----------------------------
    // 6. Execute EVM block
    // -----------------------------
    println!("cycle-tracker-start: execute EVM block");

    let executor = EthClientExecutor::eth(
        Arc::new(
            (&client_executor_input.genesis)
                .try_into()
                .expect("invalid genesis block"),
        ),
        client_executor_input.custom_beneficiary,
    );

    let header = executor
        .execute(client_executor_input)
        .expect("EVM block execution failed");

    println!("cycle-tracker-end: execute EVM block");

    // -----------------------------
    // 7. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let output = EvmBlockExecOutput {
        blob_commitment: blob.commitment.into(),
        header_hash: header.hash_slow().into(),
        prev_header_hash: header.parent_hash.into(),
        celestia_header_hash: celestia_header
            .hash()
            .as_bytes()
            .try_into()
            .expect("celestia_header_hash must be exactly 32 bytes"),
        prev_celestia_header_hash: celestia_header
            .last_block_id
            .unwrap()
            .hash
            .as_bytes()
            .try_into()
            .expect("prev_celestia_header_hash must be exactly 32 bytes"),
        new_height: header.number,
        new_state_root: header.state_root.into(),
        prev_height: header.number - 1,
        prev_state_root: trusted_state_root
            .try_into()
            .expect("prev_state_root must be exactly 32 bytes"),
    };

    sp1_zkvm::io::commit(&output);

    println!("cycle-tracker-end: commit public outputs");
}
