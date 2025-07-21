//! An SP1 program that verifies inclusion of EVM reth blocks in the Celestia data availability network
//! and executes their state transition functions.
//!
//! ## Functionality
//!
//! The program accepts the following inputs:
//! - Celestia block header and associated data availability header (DAH).
//! - Namespace
//! - Blobs
//! - Sequencer Public Key
//! - NamespaceProofs
//! - EthClientExecutorInputs (RSP - state transition function)
//! - Trusted Height
//! - Trusted State Root
//!
//! It performs the following steps:
//! 1. Deserializes the program inputs.
//! 2. Verifies completeness of the namespace using the provided blobs.
//! 3. Executes the EVM blocks via the state transition function.
//! 4. Filters blobs to SignedData and verifies the sequencer signature.
//! 5. Verifies equivalency between the EVM block data and blob data via SignedData.
//! 6. Commits a [`BlockExecOutput`] struct to the program outputs.
//!
//! The program commits the following fields to the program output:
//! - Celestia block header hash
//! - Previous Celestia block header hash
//! - New Height
//! - New State Root
//! - Trusted Height
//! - Trusted State Root
//! - Namespace
//! - Public Key
#![no_main]

sp1_zkvm::entrypoint!(main);

use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;

use alloy_consensus::{proofs, BlockHeader};
use alloy_primitives::B256;
use alloy_rlp::Decodable;
use bytes::Bytes;
use celestia_types::nmt::{Namespace, NamespaceProof, NamespacedHash};
use celestia_types::Blob;
use celestia_types::DataAvailabilityHeader;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use evm_exec_types::BlockExecOutput;
use nmt_rs::NamespacedSha2Hasher;
use prost::Message;
use reth_primitives::TransactionSigned;
use rollkit_types::v1::{Data, SignedData};
use rsp_client_executor::{
    executor::EthClientExecutor,
    io::{EthClientExecutorInput, WitnessInput},
};
use tendermint::block::Header;

pub fn main() {
    // -----------------------------
    // 0. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-start: deserialize inputs");

    let celestia_header_raw: Vec<u8> = sp1_zkvm::io::read_vec();
    let celestia_header: Header =
        serde_cbor::from_slice(&celestia_header_raw).expect("failed to deserialize celestia header");

    let dah: DataAvailabilityHeader = sp1_zkvm::io::read();

    let blobs_raw: Vec<u8> = sp1_zkvm::io::read_vec();
    let blobs: Vec<Blob> = serde_cbor::from_slice(&blobs_raw).expect("failed to deserialize blob data");

    let pub_key: Vec<u8> = sp1_zkvm::io::read_vec();
    let namespace: Namespace = sp1_zkvm::io::read();
    let proofs: Vec<NamespaceProof> = sp1_zkvm::io::read();

    let executor_inputs: Vec<EthClientExecutorInput> = sp1_zkvm::io::read();

    let trusted_height: u64 = sp1_zkvm::io::read();
    let trusted_root: B256 = sp1_zkvm::io::read();

    println!("cycle-tracker-end: deserialize inputs");

    // -----------------------------
    // 1. Verify namespace inclusion and completeness
    // -----------------------------
    println!("cycle-tracker-start: verify namespace data");

    assert_eq!(
        celestia_header.data_hash.unwrap(),
        dah.hash(),
        "DataHash mismatch for DataAvailabilityHeader"
    );

    let mut roots = Vec::<&NamespacedHash>::new();
    for row_root in dah.row_roots() {
        if row_root.contains::<NamespacedSha2Hasher<29>>(namespace.into()) {
            roots.push(row_root);
        }
    }

    if roots.is_empty() {
        assert!(blobs.is_empty(), "Blobs must be empty if no roots contain namespace");
        assert!(proofs.is_empty(), "Proofs must be empty if no roots contain namespace");
    }

    let blob_data: Vec<[u8; 512]> = blobs
        .iter()
        .flat_map(|blob| {
            blob.to_shares()
                .unwrap()
                .into_iter()
                .map(|share| share.as_ref().try_into().unwrap())
        })
        .collect();

    let mut cursor = 0;
    for (proof, root) in proofs.iter().zip(roots) {
        let share_count = (proof.end_idx() - proof.start_idx()) as usize;
        let end = cursor + share_count;

        let raw_leaves = &blob_data[cursor..end];

        proof
            .verify_complete_namespace(root, raw_leaves, namespace.into())
            .expect("Failed to verify proof");

        cursor = end;
    }

    println!("cycle-tracker-end: verify namespace data");

    // -----------------------------
    // 2. Execute the EVM block inputs
    // -----------------------------
    println!("cycle-tracker-start: execute EVM blocks");

    let mut headers = Vec::with_capacity(executor_inputs.len());
    if headers.capacity() != 0 {
        let first_input = executor_inputs.first().unwrap();

        assert_eq!(
            trusted_root,
            first_input.state_anchor(),
            "State anchor must be equal to trusted root"
        );

        assert!(
            trusted_height <= first_input.parent_header().number(),
            "Trusted height must be less than or equal to parent header height",
        );

        let executor = EthClientExecutor::eth(
            Arc::new((&first_input.genesis).try_into().expect("invalid genesis block")),
            first_input.custom_beneficiary,
        );

        for input in &executor_inputs {
            let header = executor.execute(input.clone()).expect("EVM block execution failed");
            headers.push(header);
        }
    }

    println!("cycle-tracker-end: execute EVM blocks");

    // -----------------------------
    // 3. Filter SignedData blobs and verify signatures
    // -----------------------------
    println!("cycle-tracker-start: filter signed data blobs and verify signatures");

    let signed_data: Vec<SignedData> = blobs
        .into_iter()
        .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
        .collect();

    let mut tx_data: Vec<Data> = Vec::new();
    for sd in signed_data {
        let signer = sd.signer.as_ref().expect("SignedData must contain signer");

        if signer.pub_key != pub_key {
            continue;
        }

        let data_bytes = sd.data.as_ref().expect("SignedData must contain data").encode_to_vec();

        verify_signature(&pub_key[4..], &data_bytes, &sd.signature).expect("Sequencer signature verification failed");

        tx_data.push(sd.data.unwrap());
    }

    // Equivocation tolerance: Filter out duplicate heights if applicable, accepting FCFS as the source of truth.
    if tx_data.len() != headers.len() {
        let mut seen = HashSet::<u64>::new();
        tx_data.retain(|data| get_height(data).map(|h| seen.insert(h)).unwrap_or(false));
    }

    tx_data.sort_by_key(|data| get_height(data).expect("Data must contain a height"));

    assert_eq!(
        tx_data.len(),
        headers.len(),
        "Headers and SignedData must be of equal length"
    );

    println!("cycle-tracker-end: filter signed data blobs and verify signatures");

    // -----------------------------
    // 4. Verify blob equivalency
    // -----------------------------
    println!("cycle-tracker-start: verify blob-header equivalency");

    for (header, data) in headers.iter().zip(tx_data) {
        let mut txs = Vec::with_capacity(data.txs.len());
        for tx_bytes in data.txs {
            let tx = TransactionSigned::decode(&mut tx_bytes.as_slice()).expect("Failed decoding transaction");
            txs.push(tx);
        }

        let root = proofs::calculate_transaction_root(&txs);

        assert_eq!(
            root, header.transactions_root,
            "Calculated root must be equal to header transactions root"
        );
    }

    println!("cycle-tracker-end: verify blob-header equivalency");

    // -----------------------------
    // 5. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(trusted_height);
    let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(trusted_root);

    let output = BlockExecOutput {
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
        new_height,
        new_state_root: new_state_root.into(),
        prev_height: trusted_height,
        prev_state_root: trusted_root.into(),
        namespace,
        public_key: pub_key,
    };

    sp1_zkvm::io::commit(&output);

    println!("cycle-tracker-end: commit public outputs");
}

fn get_height(data: &Data) -> Option<u64> {
    data.metadata.as_ref().map(|m| m.height)
}

fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Box<dyn Error>> {
    let pub_key: [u8; 32] = public_key
        .try_into()
        .map_err(|e| format!("Public key must be 32 bytes for Ed25519: {e}"))?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key).map_err(|e| format!("Invalid Ed25519 public key: {e}"))?;

    let signature = Signature::from_slice(signature).map_err(|e| format!("Invalid Ed25519 signature: {e}"))?;

    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("Signature verification failed: {e}"))?;

    Ok(())
}
