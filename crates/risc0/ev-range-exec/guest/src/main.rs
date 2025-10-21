//! A Risc0 program that verifies a sequence of N `ev-exec` proofs.
//!
//! It accepts:
//! - N image IDs (32-byte digests identifying the guest program)
//! - N serialized public values (each from a `BlockExecOutput`)
//!
//! It performs:
//! 1. Proof verification for each input using Risc0's env::verify
//! 2. Sequential header verification (i.e., block continuity)
//! 3. Aggregation of metadata into a `BlockRangeExecOutput`
//!
//! It commits:
//! - The trusted block height and state root
//! - The new block height and state root
//! - The latest Celestia header hash from the sequence

#![no_main]
#![no_std]

use risc0_zkvm::guest::env;
use ev_zkevm_types::programs::block::{BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput, Buffer};

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // ------------------------------
    // 1. Deserialize inputs
    // ------------------------------
    let inputs: BlockRangeExecInput = env::read();

    assert_eq!(
        inputs.vkeys.len(),
        inputs.public_values.len(),
        "mismatch between number of image IDs and public value blobs"
    );

    let proof_count = inputs.vkeys.len();

    // ------------------------------
    // 2. Verify proofs using Risc0's composition API
    // ------------------------------
    // In Risc0, we use env::verify(image_id, journal) to verify receipts
    // The image_id is a 32-byte digest, which we receive as [u32; 8]
    // We need to convert it to the proper format

    for i in 0..proof_count {
        // Convert [u32; 8] vkey to 32-byte image_id
        let image_id: [u8; 32] = convert_vkey_to_image_id(&inputs.vkeys[i]);

        // Verify the receipt using Risc0's composition API
        // env::verify expects the image_id and the journal (public values)
        env::verify(image_id, &inputs.public_values[i]).expect("Failed to verify receipt");
    }

    // ------------------------------
    // 3. Parse public values into outputs
    // ------------------------------
    let outputs: Vec<BlockExecOutput> = inputs
        .public_values
        .iter()
        .map(|bytes| {
            let mut buffer = Buffer::from(bytes);
            buffer.read::<BlockExecOutput>()
        })
        .collect();

    // ------------------------------
    // 4. Verify sequential headers (EVM and Celestia)
    // ------------------------------
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

    // ------------------------------
    // 5. Build and commit outputs
    // ------------------------------
    let first = outputs.first().expect("No outputs provided");
    let last = outputs.last().expect("No outputs provided");

    let output = BlockRangeExecOutput {
        celestia_header_hash: last.celestia_header_hash,
        trusted_height: first.prev_height,
        trusted_state_root: first.prev_state_root,
        new_state_root: last.new_state_root,
        new_height: last.new_height,
        namespace: last
            .namespace
            .as_bytes()
            .try_into()
            .expect("namespace must be 29 bytes"),
        public_key: last.public_key,
    };

    env::commit(&output);
}

/// Convert SP1-style [u32; 8] vkey format to Risc0's 32-byte image_id
fn convert_vkey_to_image_id(vkey: &[u32; 8]) -> [u8; 32] {
    let mut image_id = [0u8; 32];
    for (i, &word) in vkey.iter().enumerate() {
        let bytes = word.to_le_bytes();
        image_id[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }
    image_id
}
