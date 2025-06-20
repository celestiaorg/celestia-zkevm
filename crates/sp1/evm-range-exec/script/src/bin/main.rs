//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! RUST_LOG=info cargo run -p evm-range-exec-script --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p evm-range-exec-script --release -- --prove
//! ```
use std::error::Error;
use std::fs;
use std::path::Path;

use clap::Parser;
use evm_exec_types::EvmRangeExecOutput;
use sp1_sdk::{include_elf, HashableKey, ProverClient, SP1Proof, SP1ProofWithPublicValues, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let client = ProverClient::from_env();

    let mut stdin = SP1Stdin::new();
    write_proof_inputs(&mut stdin)?;

    if args.execute {
        // Execute the program.
        let (output, report) = client.execute(EVM_RANGE_EXEC_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let range_exec_output: EvmRangeExecOutput = bincode::deserialize(output.as_slice())?;
        println!("Outputs: {}", serde_json::to_string_pretty(&range_exec_output)?);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(EVM_RANGE_EXEC_ELF);

        // Generate the proof.
        let proof = client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Save the proof and reload.
        proof.save("testdata/groth16-proof.bin")?;
        let deserialized_proof = SP1ProofWithPublicValues::load("testdata/groth16-proof.bin")?;

        // Verify the proof.
        client.verify(&deserialized_proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }

    Ok(())
}

/// write_proof_inputs writes the program inputs to provided SP1Stdin
fn write_proof_inputs(stdin: &mut SP1Stdin) -> Result<(), Box<dyn Error>> {
    let proofs_dir = Path::new("testdata/proofs");
    let mut paths: Vec<_> = fs::read_dir(proofs_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "bin"))
        .collect();

    paths.sort();

    let proofs: Vec<SP1ProofWithPublicValues> = paths
        .iter()
        .map(|path| SP1ProofWithPublicValues::load(path))
        .collect::<Result<_, _>>()?;

    let client = ProverClient::from_env();
    let (_, vk) = client.setup(EVM_EXEC_ELF);

    let vkeys = vec![vk.hash_u32(); proofs.len()];
    stdin.write::<Vec<[u32; 8]>>(&vkeys);

    let public_inputs = proofs
        .iter()
        .map(|proof| proof.public_values.to_vec())
        .collect::<Vec<_>>();

    stdin.write::<Vec<Vec<u8>>>(&public_inputs);

    for proof in &proofs {
        let SP1Proof::Compressed(ref proof) = proof.proof else {
            panic!()
        };

        stdin.write_proof(*proof.clone(), vk.vk.clone());
    }

    Ok(())
}
