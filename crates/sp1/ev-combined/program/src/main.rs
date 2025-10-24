//! An SP1 program that verifies a sequence of N `ev-exec` proofs.
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

use ev_zkevm_types::programs::{block::EvCombinedInput, combined::verify_combined};

pub fn main() {
    let inputs: EvCombinedInput = sp1_zkvm::io::read::<EvCombinedInput>();
    let output = verify_combined(inputs).expect("failed to verify combined");
    sp1_zkvm::io::commit(&output);
}
