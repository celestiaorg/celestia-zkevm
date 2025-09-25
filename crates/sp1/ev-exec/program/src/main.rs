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
use celestia_types::nmt::{NamespacedHash, EMPTY_LEAVES};
use celestia_types::Blob;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use ev_types::v1::{Data, SignedData};
use ev_zkevm_types::programs::block::{BlockExecInput, BlockExecOutput};
use nmt_rs::NamespacedSha2Hasher;
use prost::Message;
use reth_primitives::TransactionSigned;
use rsp_client_executor::{executor::EthClientExecutor, io::WitnessInput};
use tendermint::block::Header;

pub fn main() {
    // -----------------------------
    // 0. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-report-start: deserialize inputs");

    let inputs: BlockExecInput = sp1_zkvm::io::read::<BlockExecInput>();
    let celestia_header: Header =
        serde_cbor::from_slice(&inputs.header_raw).expect("failed to deserialize celestia header");
    let blobs: Vec<Blob> = serde_cbor::from_slice(&inputs.blobs_raw).expect("failed to deserialize blob data");

    println!("cycle-tracker-report-end: deserialize inputs");

    // -----------------------------
    // 1. Verify namespace inclusion and completeness
    // -----------------------------
    println!("cycle-tracker-report-start: verify namespace data");

    assert_eq!(
        celestia_header.data_hash.unwrap(),
        inputs.dah.hash(),
        "DataHash mismatch for DataAvailabilityHeader"
    );

    let mut roots = Vec::<&NamespacedHash>::new();
    for row_root in inputs.dah.row_roots() {
        if row_root.contains::<NamespacedSha2Hasher<29>>(inputs.namespace.into()) {
            roots.push(row_root);
        }
    }

    assert_eq!(
        roots.len(),
        inputs.proofs.len(),
        "Number of proofs must equal the number of row roots"
    );

    if roots.is_empty() {
        assert!(blobs.is_empty(), "Blobs must be empty if no roots contain namespace");
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
    for (proof, root) in inputs.proofs.iter().zip(roots) {
        if proof.is_of_absence() {
            proof
                .verify_complete_namespace(root, EMPTY_LEAVES, inputs.namespace.into())
                .expect("Failed to verify proof");
            break;
        }
        let share_count = (proof.end_idx() - proof.start_idx()) as usize;
        let end = cursor + share_count;

        let raw_leaves = &blob_data[cursor..end];

        proof
            .verify_complete_namespace(root, raw_leaves, inputs.namespace.into())
            .expect("Failed to verify proof");

        cursor = end;
    }

    println!("cycle-tracker-report-end: verify namespace data");

    // -----------------------------
    // 2. Execute the EVM block inputs
    // -----------------------------
    println!("cycle-tracker-report-start: execute EVM blocks");

    let mut headers = Vec::with_capacity(inputs.executor_inputs.len());
    if headers.capacity() != 0 {
        let first_input = inputs.executor_inputs.first().unwrap();

        assert_eq!(
            inputs.trusted_root,
            first_input.state_anchor(),
            "State anchor must be equal to trusted root"
        );

        assert!(
            inputs.trusted_height <= first_input.parent_header().number(),
            "Trusted height must be less than or equal to parent header height",
        );

        let executor = EthClientExecutor::eth(
            Arc::new((&first_input.genesis).try_into().expect("invalid genesis block")),
            first_input.custom_beneficiary,
        );

        for input in &inputs.executor_inputs {
            let header = executor.execute(input.clone()).expect("EVM block execution failed");
            headers.push(header);
        }
    }

    println!("cycle-tracker-report-end: execute EVM blocks");

    // -----------------------------
    // 3. Filter SignedData blobs and verify signatures
    // -----------------------------
    println!("cycle-tracker-report-start: filter signed data blobs and verify signatures");

    let signed_data: Vec<SignedData> = blobs
        .into_iter()
        .filter_map(|blob| SignedData::decode(blob.data.as_slice()).ok())
        .collect();

    let mut tx_data: Vec<Data> = Vec::new();
    for sd in signed_data {
        let signer = sd.signer.as_ref().expect("SignedData must contain signer");

        // NOTE: Trim 4 byte Protobuf encoding prefix
        if signer.pub_key[4..] != inputs.pub_key {
            continue;
        }

        let mut data_bytes = Vec::new();
        prost::Message::encode(sd.data.as_ref().expect("SignedData must contain data"), &mut data_bytes)
            .expect("Failed to encode data");

        verify_signature(&inputs.pub_key, &data_bytes, &sd.signature).expect("Sequencer signature verification failed");

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

    println!("cycle-tracker-report-end: filter signed data blobs and verify signatures");

    // -----------------------------
    // 4. Verify blob equivalency
    // -----------------------------
    println!("cycle-tracker-report-start: verify blob-header equivalency");

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

    println!("cycle-tracker-report-end: verify blob-header equivalency");

    // -----------------------------
    // 5. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-report-start: commit public outputs");

    let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(inputs.trusted_height);
    let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(inputs.trusted_root);

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
        prev_height: inputs.trusted_height,
        prev_state_root: inputs.trusted_root.into(),
        namespace: inputs.namespace,
        public_key: inputs.pub_key.try_into().expect("public key must be exactly 32 bytes"),
    };
    sp1_zkvm::io::commit(&output);
    println!("cycle-tracker-report-end: commit public outputs");
}

fn get_height(data: &Data) -> Option<u64> {
    data.metadata.as_ref().map(|m| m.height)
}

fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("cycle-tracker-report-start: verify ed25519 signature");

    let pub_key: [u8; 32] = public_key
        .try_into()
        .map_err(|e| format!("Public key must be 32 bytes for Ed25519: {e}"))?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key).map_err(|e| format!("Invalid Ed25519 public key: {e}"))?;

    let signature = Signature::from_slice(signature).map_err(|e| format!("Invalid Ed25519 signature: {e}"))?;

    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("Signature verification failed: {e}"))?;

    println!("cycle-tracker-report-end: verify ed25519 signature");
    Ok(())
}
