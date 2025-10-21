//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! RUST_LOG=info cargo run -p ev-range-exec-script --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p ev-range-exec-script --release -- --prove
//! ```
use std::error::Error;
use std::fs;
use std::path::Path;

use clap::Parser;
use ev_zkevm_types::programs::block::{BlockRangeExecInput, BlockRangeExecOutput};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, HashableKey, ProverClient, SP1Proof, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_EXEC_ELF: &[u8] = include_elf!("ev-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_RANGE_EXEC_ELF: &[u8] = include_elf!("ev-range-exec-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Run the program in execute mode")]
    execute: bool,

    #[arg(long, help = "Run the program in prove mode")]
    prove: bool,

    #[arg(long, help = "Output file for benchmark report in JSON format")]
    output_file: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BenchmarkReport {
    pub total_proofs: u64,
    pub total_gas: u64,
    pub total_instruction_count: u64,
    pub total_syscall_count: u64,
    pub cycle_tracker_results: HashMap<String, u64>,
}

fn main() -> Result<(), Box<dyn Error>> {
    sp1_sdk::utils::setup_logger();
    dotenvy::dotenv().ok();

    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let client = ProverClient::from_env();

    let mut stdin = SP1Stdin::new();
    let num_proofs = write_proof_inputs(&mut stdin)?;

    if args.execute {
        // Execute the program.
        let (output, report) = client.execute(EV_RANGE_EXEC_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let range_exec_output: BlockRangeExecOutput = bincode::deserialize(output.as_slice())?;
        println!("Outputs: {range_exec_output}");

        // Record the total gas and number of cycles executed.
        println!("Total gas: {}", report.gas.unwrap());
        println!("Total instruction count: {}", report.total_instruction_count());
        println!("Total syscall count: {}", report.total_syscall_count());

        // If an output opt is provided then write the results to JSON.
        if let Some(output_file) = args.output_file {
            let benchmark_report = BenchmarkReport {
                total_proofs: num_proofs as u64,
                total_gas: report.gas.unwrap(),
                total_instruction_count: report.total_instruction_count(),
                total_syscall_count: report.total_syscall_count(),
                cycle_tracker_results: report.cycle_tracker,
            };

            let json = serde_json::to_string_pretty(&benchmark_report).unwrap();
            fs::write(format!("testdata/benchmarks/{output_file}"), json)?;
        }
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(EV_RANGE_EXEC_ELF);

        // Generate the proof.
        let proof = client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .expect("failed to generate proof");

        fs::write("proof.bin", proof.bytes())?;
        fs::write("public_values.bin", proof.public_values.as_slice())?;
        let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
        let prev_header_hash = hex::encode(public_values.prev_celestia_header_hash);
        let prev_height = public_values.prev_celestia_height;
        println!("Prev header hash: {prev_header_hash}, Prev height: {prev_height}");
        println!("Public values: {public_values:?}");

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
fn write_proof_inputs(stdin: &mut SP1Stdin) -> Result<usize, Box<dyn Error>> {
    let proofs_dir = Path::new("testdata/proofs");
    let mut paths: Vec<_> = fs::read_dir(proofs_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "bin"))
        .collect();

    paths.sort();

    let proofs: Vec<SP1ProofWithPublicValues> = paths
        .iter()
        .map(SP1ProofWithPublicValues::load)
        .collect::<Result<_, _>>()?;

    let vk: SP1VerifyingKey = bincode::deserialize(&fs::read("testdata/vkeys/ev-exec-vkey.bin")?)?;
    let vkeys = vec![vk.hash_u32(); proofs.len()];

    let public_inputs = proofs
        .iter()
        .map(|proof| proof.public_values.to_vec())
        .collect::<Vec<_>>();

    let input = BlockRangeExecInput {
        vkeys,
        public_values: public_inputs,
        trusted_celestia_height: 0,
        trusted_celestia_root: [0; 32],
    };
    stdin.write(&input);

    for proof in &proofs {
        let SP1Proof::Compressed(ref proof) = proof.proof else {
            panic!()
        };
        stdin.write_proof(*proof.clone(), vk.vk.clone());
    }

    Ok(proofs.len())
}
