//! An SP1 program that proves the state of the Hyperlane Merkle Tree against an EVM state root.
//!
//! ## Functionality
//!
//! The program accepts the following inputs:
//! - state root
//! - contract address of the MerkleTreeHook contract
//! - storage proof of the 32 branch nodes and the count of the incremental tree
//!
//! It performs the following steps:
//! Verify the branch nodes and the count of the incremental tree on-chain against the provided state root.
//! Compute the root of the incremental tree from the proven branch and count.
//! The program commits the following fields to the program output:
//! - The execution state root
//! - The MerkleTreeHook contract address
//! - The root of the incremental tree
//! - The number of leaves in the incremental tree
//!
//! Messages are then verified individually on the destination chain with a merkle proof against the committed root.

#![no_main]
use std::str::FromStr;

use alloy_primitives::{hex, Address};
use ev_zkevm_types::hyperlane::{
    io::{HyperlaneTreeInputs, HyperlaneTreeOutputs},
    merkle::MerkleTree,
};
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let inputs: HyperlaneTreeInputs = sp1_zkvm::io::read::<HyperlaneTreeInputs>();
    let tree = inputs.verify();
    let tree_root = tree
        .root_with_ctx(&MerkleTree::zero_hashes())
        .expect("Failed to compute tree root");

    sp1_zkvm::io::commit(&HyperlaneTreeOutputs {
        state_root: hex::decode(&inputs.state_root).unwrap().try_into().unwrap(),
        merkle_tree_address: *Address::from_str(&inputs.contract).unwrap().into_word(),
        tree_root: hex::decode(tree_root).unwrap().try_into().unwrap(),
        count: tree.count as u64,
    });
}
