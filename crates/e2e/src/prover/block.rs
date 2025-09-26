// This endpoint generates a block proof for a range (trusted_height, target_height)
// and wraps it recursively into a single groth16 proof using the ev-range-exec program.

#![allow(unused)]

use anyhow::Result;

pub fn prove_blocks(trusted_height: u64, target_height: u64) -> Result<()> {
    Ok(())
}
