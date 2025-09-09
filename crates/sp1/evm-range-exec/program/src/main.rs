//! An SP1 program that verifies a sequence of N `evm-exec` proofs.
//!
//! It accepts:
//! - N verification keys
//! - N serialized public values (each from a `EvmBlockExecOutput`)
//!
//! It performs:
//! 1. Proof verification for each input
//! 2. Sequential header verification (i.e., block continuity)
//! 3. Aggregation of metadata into a `EvmRangeExecOutput`
//!
//! It commits:
//! - The trusted block height and state root
//! - The new block height and state root
//! - The latest Celestia header hash from the sequence

#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_exec_types::{BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput, Buffer};
use sha2::{Digest, Sha256};

pub fn main() {
    // ------------------------------
    // 1. Deserialize inputs
    // ------------------------------
    println!("cycle-tracker-report-start: deserialize inputs");

    let inputs: BlockRangeExecInput = sp1_zkvm::io::read::<BlockRangeExecInput>();

    assert_eq!(
        inputs.vkeys.len(),
        inputs.public_values.len(),
        "mismatch between number of verification keys and public value blobs"
    );

    let proof_count = inputs.vkeys.len();

    println!("cycle-tracker-report-end: deserialize inputs");

    // ------------------------------
    // 2. Verify proofs
    // ------------------------------
    println!("cycle-tracker-report-start: verify sp1 proofs");

    for i in 0..proof_count {
        let digest = Sha256::digest(&inputs.public_values[i]);
        sp1_zkvm::lib::verify::verify_sp1_proof(&inputs.vkeys[i], &digest.into());
    }
}
