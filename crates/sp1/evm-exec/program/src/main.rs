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

use celestia_types::ShareProof;
use evm_exec_types::EvmBlockExecOutput;
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
    // 3. Verify Blob inclusion if new Header contains transactions
    // -----------------------------
    println!("cycle-tracker-start: verify blob inclusion for headers");

    // Filters headers with empty transaction roots
    headers.retain(|header| !header.transaction_root_is_empty());
    if headers.len() != blob_proofs.len() {
        panic!("Number of headers with blob tx data do not match");
    }

    for (header, blob_proof) in headers.iter().zip(blob_proofs) {
        blob_proof.verify(celestia_header.data_hash.unwrap()).expect(&format!(
            "ShareProof verification failed for block number {}",
            header.number
        ));

        // TODO: Verify blob tx data equivalency against header.transactions_root
        // https://github.com/celestiaorg/celestia-zkevm-hl-testnet/issues/68
    }

    println!("cycle-tracker-end: verify blob inclusion for headers");
    // -----------------------------
    // 4. Build and commit outputs
    // -----------------------------
    println!("cycle-tracker-start: commit public outputs");

    let output = EvmBlockExecOutput {
        new_header_hash: headers.last().unwrap().hash_slow().into(),
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
