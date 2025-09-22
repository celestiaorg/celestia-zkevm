//! A utility for parsing a `SP1ProofWithPublicValues` into its constituent components:
//! the Groth16 proof and the associated public inputs.
//!
//! The script expects an existing file at `testdata/groth16-proof.bin` and will output:
//! - `testdata/proof.bin`: the Groth16 proof bytes
//! - `testdata/sp1-inputs.bin`: the serialized public inputs
//!
//! ## Usage
//! Run this script from the root of the repository with:
//! ```shell
//! RUST_LOG=info cargo run -p ev-range-exec-script --bin parser --release
//! ```
use std::error::Error;
use std::fs;

use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_EXEC_ELF: &[u8] = include_elf!("ev-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_RANGE_EXEC_ELF: &[u8] = include_elf!("ev-range-exec-program");

fn main() -> Result<(), Box<dyn Error>> {
    sp1_sdk::utils::setup_logger();
    dotenvy::dotenv().ok();

    let client = ProverClient::from_env();

    // Setup the program for proving.
    let (_, vk) = client.setup(EV_RANGE_EXEC_ELF);

    let proof = SP1ProofWithPublicValues::load("testdata/groth16-proof.bin")?;

    // Verify the proof.
    client.verify(&proof, &vk).expect("failed to verify proof");
    println!("Successfully verified proof!");

    let proof_bytes = proof.bytes();
    let sp1_public_values = proof.public_values.clone();

    // Save the Groth16 proof components
    fs::write("testdata/proof.bin", &proof_bytes)?;

    // Save public inputs (the committed values from the circuit)
    fs::write("testdata/sp1-inputs.bin", sp1_public_values.as_slice())?;

    println!("Saved groth16 proof components to testdata directory");

    Ok(())
}
