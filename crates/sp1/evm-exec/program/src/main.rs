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
use std::sync::Arc;

use alloy_consensus::Header as EvmHeader;
use bytes::Bytes;
use celestia_types::Blob;
use celestia_types::Share;
use celestia_types::ShareProof;
use evm_exec_types::EvmBlockExecOutput;
use prost::Message;
use reth_primitives::alloy_primitives::private::alloy_rlp::Decodable;
use reth_primitives::proofs;
use reth_primitives::TransactionSigned;
use rollkit_types::v1::SignedData;
use rsp_client_executor::{
    executor::EthClientExecutor,
    io::{EthClientExecutorInput, WitnessInput},
};
use tendermint::block::Header;

pub fn main() {
    // -----------------------------
    // 1. Deserialize inputs
    // -----------------------------
    println!("cycle-tracker-start: deserialize inputs");

    let executor_inputs: Vec<EthClientExecutorInput> = sp1_zkvm::io::read();

    let celestia_header_raw: Vec<u8> = sp1_zkvm::io::read_vec();
    let celestia_header: Header =
        serde_cbor::from_slice(&celestia_header_raw).expect("failed to deserialize celestia header");

    let blob_proofs: Vec<ShareProof> = sp1_zkvm::io::read();

    println!("cycle-tracker-end: deserialize inputs");

    // -----------------------------
    // 2. Execute the EVM block inputs
    // -----------------------------

    println!("cycle-tracker-start: execute EVM blocks");

    let executor = EthClientExecutor::eth(
        Arc::new((&executor_inputs[0].genesis).try_into().expect("invalid genesis block")),
        executor_inputs[0].custom_beneficiary,
    );

    let mut headers = Vec::with_capacity(executor_inputs.len());
    for input in &executor_inputs {
        let header = executor.execute(input.clone()).expect("EVM block execution failed");
        headers.push(header);
    }

    println!("cycle-tracker-end: execute EVM blocks");

    // -----------------------------
    // 3. Filter headers and verify blob inclusion
    // -----------------------------
    println!("cycle-tracker-start: verify blob inclusion for headers");

    // Filters headers with empty transaction roots
    let filtered_headers: Vec<EvmHeader> = headers
        .iter()
        .cloned()
        .filter(|header| !header.transaction_root_is_empty())
        .collect();

    if filtered_headers.len() != blob_proofs.len() {
        panic!("Number of headers with blob tx data do not match");
    }

    for (header, blob_proof) in filtered_headers.iter().zip(blob_proofs) {
        blob_proof.verify(celestia_header.data_hash.unwrap()).expect(&format!(
            "ShareProof verification failed for block number {}",
            header.number
        ));

        // TODO: Verify blob tx data equivalency against header.transactions_root
        // https://github.com/celestiaorg/celestia-zkevm-hl-testnet/issues/68
        // let raw_blob: Vec<u8> = blob_proof
        //     .data
        //     .iter()
        //     .flat_map(|chunk| chunk.as_slice()) // &[u8;512] -> &[u8]
        //     .copied() // copy the bytes
        //     .collect();
        // let signed_data = match Data::decode(blob.data.as_slice()) {
        //     Ok(data) => data,
        //     Err(e) => panic!("Failed decoding blob data: {e}"),
        // };

        // let mut txs = Vec::with_capacity(signed_data.txs.len());
        // for tx_bytes in signed_data.txs {
        //     let tx = TransactionSigned::decode(&mut tx_bytes.as_slice()).expect("Failed decoding transaction");
        //     txs.push(tx);
        // }

        let shares: Vec<Share> = blob_proof
            .shares()
            .iter()
            .map(|b| Share::from_raw(b).expect("Failed to parse raw shares"))
            .collect();

        let blob = Blob::reconstruct(&shares, celestia_types::AppVersion::V3).expect("Failed to reconstruct blob");

        let signed_data = match SignedData::decode(Bytes::from(blob.data)) {
            Ok(data) => data,
            Err(e) => panic!("Failed decoding blob data: {e}"),
        };

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

    let first = headers.first().unwrap();
    let last = headers.last().unwrap();

    let output = EvmBlockExecOutput {
        new_header_hash: last.hash_slow().into(),
        prev_header_hash: first.parent_hash.into(),
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
        new_height: last.number,
        new_state_root: last.state_root.into(),
        prev_height: first.number - 1,
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
