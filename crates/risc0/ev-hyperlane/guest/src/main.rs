//! A Risc0 guest program that verifies the correctness of hyperlane messages against the on-chain Merkle Tree state.
//!
//! ## Functionality
//!
//! The program accepts the following inputs:
//! - state root
//! - contract address of the MerkleTreeHook contract
//! - messages
//! - branch proof
//! - snapshot of the Merkle Tree after previous inserts (or the default Merkle Tree)
//!
//! It performs the following steps:
//! - Verify the latest branch of the incremental tree on-chain against the provided state root.
//! - Insert the message ids into the snapshot.
//! - Assert equality between the branch nodes of the snapshot and the branch nodes of the incremental tree on-chain.
//!
//! The program commits the following fields to the program output:
//! - The execution state root
//! - The message ids

#![no_main]
#![no_std]

use risc0_zkvm::guest::env;
use ev_zkevm_types::programs::hyperlane::types::{HyperlaneMessageInputs, HyperlaneMessageOutputs};

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Read inputs from host
    let mut inputs: HyperlaneMessageInputs = env::read();

    // Verify the hyperlane message inputs
    inputs.verify();

    // Create and commit outputs
    let output = HyperlaneMessageOutputs::new(
        alloy_primitives::hex::decode(inputs.state_root)
            .unwrap()
            .try_into()
            .unwrap(),
        inputs
            .messages
            .iter()
            .map(|m| alloy_primitives::hex::decode(m.id()).unwrap().try_into().unwrap())
            .collect(),
    );

    // Commit outputs to journal
    env::commit(&output);
}
