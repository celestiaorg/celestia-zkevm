//! An SP1 program that verifies a sequence of N `evm-exec` proofs.
//!
//! It accepts:
//! - N verification keys
//! - N serialized public values (each from a `EvmBlockExecOutput`)
//!
//! It performs:
//! 1. Proof verification for each input
//! 2. Header linkage verification (i.e., block continuity)
//! 3. Aggregation of metadata into a `EvmRangeExecOutput`
//!
//! It commits:
//! - The oldest and newest EVM block header hashes
//! - The final block height and state root
//! - All Celestia header hashes from the sequence

#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_exec_types::{Buffer, EvmBlockExecOutput, EvmRangeExecOutput};
use sha2::{Digest, Sha256};

pub fn main() {
    // ------------------------------
    // 1. Deserialize inputs
    // ------------------------------

    let vkeys = sp1_zkvm::io::read::<Vec<[u32; 8]>>();
    let public_values = sp1_zkvm::io::read::<Vec<Vec<u8>>>();

    assert_eq!(
        vkeys.len(),
        public_values.len(),
        "mismatch between number of verification keys and public value blobs"
    );

    let proof_count = vkeys.len();

    // ------------------------------
    // 2. Verify proofs
    // ------------------------------

    for i in 0..proof_count {
        let digest = Sha256::digest(&public_values[i]);
        sp1_zkvm::lib::verify::verify_sp1_proof(&vkeys[i], &digest.into());
    }

    // ------------------------------
    // 3. Parse public values into outputs
    // ------------------------------

    let outputs: Vec<EvmBlockExecOutput> = public_values
        .iter()
        .map(|bytes| {
            let mut buffer = Buffer::from(bytes);
            buffer.read::<EvmBlockExecOutput>()
        })
        .collect();

    // ------------------------------
    // 4. Verify sequential headers
    // ------------------------------

    for window in outputs.windows(2).enumerate() {
        let (i, pair) = window;
        let (prev, curr) = (&pair[0], &pair[1]);
        assert_eq!(
            curr.prev_header_hash,
            prev.header_hash,
            "verify sequential headers failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.header_hash,
            curr.prev_header_hash
        );
    }

    // ------------------------------
    // 5. Build and commit outputs
    // ------------------------------

    let first = outputs.first().expect("No outputs provided");
    let last = outputs.last().expect("No outputs provided");

    let celestia_header_hashes: Vec<_> = outputs.iter().map(|o| o.celestia_header_hash).collect();

    let range_output = EvmRangeExecOutput {
        celestia_header_hashes: celestia_header_hashes,
        celestia_header_hash: [0u8; 32], // TODO: assign this correctly
        trusted_height: first.prev_height,
        trusted_state_root: first.prev_state_root,
        new_state_root: last.new_state_root,
        new_height: last.new_height,
    };

    sp1_zkvm::io::commit(&range_output);
}
