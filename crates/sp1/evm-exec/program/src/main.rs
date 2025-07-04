//! An SP1 program that verifies inclusion of an EVM reth block in the Celestia data availability network
//! and executes its state transition function.
//!
//! 1. Accepts an EVM block STF and associated Celestia proofs.
//! 2. Executes the EVM block STF.
//! 3. Verifies that the EVM block tx data was included in the Celestia block, if applicable.
//! 4. Commits the resulting EVM and Celestia block metadata as public outputs.

#![no_main]

sp1_zkvm::entrypoint!(main);

use std::sync::Arc;

use celestia_types::{blob::Blob, hash::Hash, AppVersion, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::{simple_merkle::proof::Proof, simple_merkle::tree::MerkleHash, TmSha2Hasher};
use rsp_client_executor::{
    executor::EthClientExecutor,
    io::{EthClientExecutorInput, WitnessInput},
};
use tendermint::block::Header;
use tendermint_proto::Protobuf;

pub fn main() {
    // -----------------------------
    // 1. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-start: deserialize inputs");

    let executor_inputs: Vec<EthClientExecutorInput> = sp1_zkvm::io::read();

    let celestia_header_raw: Vec<u8> = sp1_zkvm::io::read_vec();
    let celestia_header: Header =
        serde_cbor::from_slice(&celestia_header_raw).expect("failed to deserialize celestia header");

    println!("cycle-tracker-end: deserialize inputs");

    // -----------------------------
    // 2. Execute the EVM block
    // -----------------------------
    println!("cycle-tracker-start: execute EVM blocks");

    let mut headers = Vec::with_capacity(executor_inputs.len());

    let executor = EthClientExecutor::eth(
        Arc::new((&executor_inputs[0].genesis).try_into().expect("invalid genesis block")),
        executor_inputs[0].custom_beneficiary,
    );

    for input in &executor_inputs {
        let header = executor.execute(input.clone()).expect("EVM block execution failed");
        headers.push(header);
    }

    println!("cycle-tracker-end: execute EVM blocks");

    // -----------------------------
    // 3. Verify Blob inclusion if new Header contains transactions
    // -----------------------------
    for header in &headers {
        if !header.transaction_root_is_empty() {
            println!("cycle-tracker-start: deserialize inputs");

            let blob_proof: KeccakInclusionToDataRootProofInput = sp1_zkvm::io::read();
            let data_hash_proof: Proof<TmSha2Hasher> = sp1_zkvm::io::read();

            println!("cycle-tracker-end: deserialize inputs");

            verify_blob_inclusion(blob_proof.clone());

            // TODO: It should be possible to only verify the data root in the celestia header once.
            // We should pass celestia_header.data_hash to verify_blob_inclusion() and use ShareProof type directly instead of
            // the KeccakInclusionToDatRootProofInput type
            let data_root = Hash::Sha256(blob_proof.data_root);
            verify_data_root_inclusion(data_root, celestia_header.clone(), data_hash_proof);
        }
    }

    // -----------------------------
    // 4. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let output = EvmBlockExecOutput {
        blob_commitment: [0u8; 32], // TODO: remove this
        header_hash: headers.last().unwrap().hash_slow().into(),
        prev_header_hash: headers.first().unwrap().parent_hash.into(),
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
        new_height: headers.last().unwrap().number,
        new_state_root: headers.last().unwrap().state_root.into(),
        prev_height: headers.first().unwrap().number - 1,
        prev_state_root: executor_inputs
            .first()
            .unwrap()
            .state_anchor()
            .try_into()
            .expect("prev_state_root must be exactly 32 bytes"),
    };

    sp1_zkvm::io::commit(&output);

    println!("cycle-tracker-end: commit public outputs");
}

/// Verifies inclusion of a Celestia blob given a `ShareProof`.
///
/// Performs the following steps:
/// 1. Reconstructs the blob from namespace and data.
/// 2. Reconstructs the `ShareProof`.
/// 3. Verifies the `ShareProof` against the provided data root.
///
/// Panics if any step fails.
fn verify_blob_inclusion(blob_proof: KeccakInclusionToDataRootProofInput) {
    println!("cycle-tracker-start: create blob");

    let blob =
        Blob::new(blob_proof.namespace_id, blob_proof.data, AppVersion::V3).expect("failed to construct Celestia blob");

    println!("cycle-tracker-end: create blob");

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
}

/// Verifies inclusion of a the given data root within the Celestia header.
///
/// Panics if any step fails.
fn verify_data_root_inclusion(data_root: Hash, header: Header, proof: Proof<TmSha2Hasher>) {
    println!("cycle-tracker-start: verify data root");

    let hasher = TmSha2Hasher {};
    proof
        .verify_range(
            header.hash().as_bytes().try_into().unwrap(),
            &[hasher.hash_leaf(&data_root.encode_vec())],
        )
        .expect("Celestia inclusion proof failed");

    println!("cycle-tracker-end: verify data root");
}
