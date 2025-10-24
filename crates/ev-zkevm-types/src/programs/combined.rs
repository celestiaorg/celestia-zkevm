use crate::programs::block::{BlockExecOutput, BlockRangeExecOutput, EvCombinedInput, verify_block};
use std::error::Error;

pub fn verify_combined(inputs: EvCombinedInput) -> Result<BlockRangeExecOutput, Box<dyn Error>> {
    let mut outputs: Vec<BlockExecOutput> = Vec::new();
    for block in inputs.blocks {
        outputs.push(verify_block(block).expect("failed to verify block"));
    }

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

    let first = outputs.first().expect("No outputs provided");
    let last = outputs.last().expect("No outputs provided");

    let output = BlockRangeExecOutput {
        prev_celestia_height: first.prev_celestia_height,
        prev_celestia_header_hash: first.prev_celestia_header_hash,
        celestia_height: first.prev_celestia_height + outputs.len() as u64,
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
    Ok(output)
}
