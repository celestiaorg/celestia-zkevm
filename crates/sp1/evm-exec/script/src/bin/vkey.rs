//! A simple utility to extract and print the verifier key for the `evm-exec-program` zkVM circuit.
//!
//! This program initializes an SP1 prover client and performs a one-time setup to compute
//! the proving and verification keys for the ELF binary. It then prints the verification key
//! in hex format (as a 32-byte hash) to stdout.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! cargo run -p evm-exec-script --bin vkey --release
//! ```
use std::fs;

use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EVM_EXEC_ELF);
    println!("evm-exec-program vkey: {}", vk.bytes32());

    let encoded = bincode::serialize(&vk)?;
    let path = "testdata/vkeys/evm-exec-vkey.bin";
    fs::write(path, encoded)?;
    println!("successfully wrote vkey to: {}", path);

    Ok(())
}
