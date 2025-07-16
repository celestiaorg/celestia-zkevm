//! An SP1 program that verifies inclusion of EVM reth blocks in the Celestia data availability network
//! and executes their state transition functions.
//!
//! ## Functionality
//!
//! This program accepts a batch of EVM execution inputs along with a single Celestia block header
//! and, if necessary, inclusion proofs for transaction blobs. It performs the following steps:
//!
//! 1. Deserializes a vector of [`EthClientExecutorInput`] values and a single Celestia block header.
//! 2. Executes the EVM block state transition function (STF) for each input block.
//! 3. If the EVM block contains transactions, verifies that the corresponding blob was included in
//!    the specified Celestia block via a [`ShareProof`].
//! 4. Commits an [`EvmBlockExecOutput`] structure as the public output of the program, containing:
//!     - Final EVM block header hash and state root,
//!     - Previous EVM header hash and state root (from the first input),
//!     - Celestia header hashes (current and previous).
//!
#![no_main]

sp1_zkvm::entrypoint!(main);

use std::collections::HashSet;
use std::sync::Arc;

use alloy_consensus::BlockHeader;
use bytes::Bytes;
use celestia_types::nmt::{Namespace, NamespaceProof, NamespacedHash};
use celestia_types::Blob;
use celestia_types::DataAvailabilityHeader;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::NamespacedSha2Hasher;
use prost::Message;
use reth_primitives::alloy_primitives::private::alloy_rlp::Decodable;
use reth_primitives::TransactionSigned;
use reth_primitives::{proofs, B256};
use rollkit_types::v1::SignedData;
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
    let namespace: Namespace = sp1_zkvm::io::read();

    let executor_inputs: Vec<EthClientExecutorInput> = sp1_zkvm::io::read();
    let blobs: Vec<Blob> = sp1_zkvm::io::read();
    let proofs: Vec<NamespaceProof> = sp1_zkvm::io::read();

    let trusted_height: u64 = sp1_zkvm::io::read();
    let trusted_root: B256 = sp1_zkvm::io::read();

    println!("cycle-tracker-end: deserialize inputs");

    // -----------------------------
    // 1. Verify namespace inclusion and completeness
    // -----------------------------
    println!("cycle-tracker-start: verify namespace");

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

    if roots.len() == 0 {
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

    println!("cycle-tracker-end: verify namespace");

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
    // 3. Verify blob equivalency
    // -----------------------------
    println!("cycle-tracker-start: verify blob equivalency for headers");

    let mut signed_data: Vec<SignedData> = blobs
        .into_iter()
        .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
        .collect();

    // Filter duplicates if length is unequal
    if signed_data.len() != headers.len() {
        let mut seen = HashSet::<u64>::new();
        signed_data.retain(|sd| signed_data_height(sd).map(|h| seen.insert(h)).unwrap_or(false));
    }

    assert_eq!(
        signed_data.len(),
        headers.len(),
        "Headers and SignedData must be of equal length"
    );

    for (header, signed_data) in headers.iter().zip(signed_data) {
        let mut txs = Vec::with_capacity(signed_data.data.clone().unwrap().txs.len());
        for tx_bytes in signed_data.data.unwrap().txs {
            let tx = TransactionSigned::decode(&mut tx_bytes.as_slice()).expect("Failed decoding transaction");
            txs.push(tx);
        }

        let root = proofs::calculate_transaction_root(&txs);

        assert_eq!(
            root, header.transactions_root,
            "Calculated root must be equal to header transactions root"
        );
    }

    println!("cycle-tracker-end: verify blob inclusion for headers");

    // -----------------------------
    // 4. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(trusted_height);
    let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(trusted_root);

    let output = EvmBlockExecOutput {
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
        new_height: new_height,
        new_state_root: new_state_root.into(),
        prev_height: trusted_height,
        prev_state_root: trusted_root.into(),
    };

    sp1_zkvm::io::commit(&output);

    println!("cycle-tracker-end: commit public outputs");
}

fn signed_data_height(sd: &SignedData) -> Option<u64> {
    sd.data.as_ref().and_then(|d| d.metadata.as_ref()).map(|m| m.height)
}
