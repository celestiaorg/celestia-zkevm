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

use ev_zkevm_types::programs::block::BlockExecInput;

pub fn main() {
    println!("cycle-tracker-report-start: read inputs");
    let inputs: BlockExecInput = sp1_zkvm::io::read::<BlockExecInput>();
    println!("cycle-tracker-report-end: read inputs");

    println!("cycle-tracker-report-start: verify and execute");
    let output = inputs.verify_and_execute().expect("Block execution verification failed");
    println!("cycle-tracker-report-end: verify and execute");

    println!("cycle-tracker-report-start: commit outputs");
    sp1_zkvm::io::commit(&output);
    println!("cycle-tracker-report-end: commit outputs");
}
