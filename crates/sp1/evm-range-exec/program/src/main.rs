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

    let inputs: BlockRangeExecInput =
        bincode::deserialize(&sp1_zkvm::io::read_vec()).expect("failed to deserialize circuit inputs");

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

    println!("cycle-tracker-report-end: verify sp1 proofs");

    // ------------------------------
    // 3. Parse public values into outputs
    // ------------------------------
    println!("cycle-tracker-report-start: decode public values from proofs");

    let outputs: Vec<BlockExecOutput> = inputs
        .public_values
        .iter()
        .map(|bytes| {
            let mut buffer = Buffer::from(bytes);
            buffer.read::<BlockExecOutput>()
        })
        .collect();

    println!("cycle-tracker-report-end: decode public values from proofs");

    // ------------------------------
    // 4. Verify sequential headers (EVM and Celestia)
    // ------------------------------
    println!("cycle-tracker-report-start: verify sequential headers");
    for window in outputs.windows(2).enumerate() {
        let (i, pair) = window;
        let (prev, curr) = (&pair[0], &pair[1]);
        assert_eq!(
            curr.prev_height,
            prev.new_height,
            "verify sequential EVM headers failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.new_height,
            curr.prev_height
        );

        assert_eq!(
            curr.prev_state_root,
            prev.new_state_root,
            "verify sequential EVM state roots failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.new_state_root,
            curr.prev_state_root
        );

        assert_eq!(
            curr.prev_celestia_header_hash,
            prev.celestia_header_hash,
            "verify sequential Celestia headers failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.celestia_header_hash,
            curr.prev_celestia_header_hash
        );

        assert_eq!(
            curr.namespace, prev.namespace,
            "unexpected namespace: expected {:?}, got {:?}",
            prev.namespace, curr.namespace
        );

        assert_eq!(
            curr.public_key, prev.public_key,
            "unexpected public key: expected {:?}, got {:?}",
            prev.public_key, curr.public_key
        );
    }

    println!("cycle-tracker-report-end: verify sequential headers");

    // ------------------------------
    // 5. Build and commit outputs
    // ------------------------------
    println!("cycle-tracker-report-start: build and commit outputs");

    let first = outputs.first().expect("No outputs provided");
    let last = outputs.last().expect("No outputs provided");

    let output = BlockRangeExecOutput {
        celestia_header_hash: last.celestia_header_hash,
        trusted_height: first.prev_height,
        trusted_state_root: first.prev_state_root,
        new_state_root: last.new_state_root,
        new_height: last.new_height,
        namespace: last.namespace,
        public_key: last.public_key,
    };

    println!("cycle-tracker-report-end: build and commit outputs");

    sp1_zkvm::io::commit(&output);
}
