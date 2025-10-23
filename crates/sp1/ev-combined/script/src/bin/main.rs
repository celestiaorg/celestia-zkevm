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

use alloy_primitives::FixedBytes;
use clap::Parser;
use ev_zkevm_types::programs::block::{BlockRangeExecOutput, EvCombinedInput};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_RANGE_EXEC_ELF: &[u8] = include_elf!("ev-combined-program");

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
    //todo: get BlockExecInputs for a bunch of blocks
    let input = EvCombinedInput {
        blocks: Vec::new(),
        trusted_height: 0,
        trusted_root: FixedBytes::from([0; 32]),
    };
    stdin.write(&input);

    Ok(input.blocks.len())
}
