//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! The program loads input files from a block-specific directory (e.g., `testdata/inputs/block-1010/`)
//! and writes the resulting proof to `testdata/proofs/proof-with-pis-<height>.bin`.
//!
//! You must provide the block number via `--height`, along with either `--execute` or `--prove`.
//! The `--trusted-height` and `--trusted-root` flags are optional, however they must be set if proving
//! an empty Celestia block, i.e. where there is no EthClientExecutorInputs.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! RUST_LOG=info cargo run -p ev-exec-script --release -- --execute --height 12 --trusted-height 18
//! --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p ev-exec-script --release -- --prove --height 12 --trusted-height 18
//! --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
//! ```
use std::env;
use std::error::Error;
use std::fs;

use alloy_primitives::FixedBytes;
use anyhow::Result;
use celestia_types::nmt::{Namespace, NamespaceProof};
use celestia_types::{Blob, DataAvailabilityHeader};
use clap::Parser;
use ev_zkevm_types::programs::block::{BlockExecInput, BlockExecOutput};
use hashbrown::HashMap;
use rsp_client_executor::io::{EthClientExecutorInput, WitnessInput};
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use tendermint::block::header::Header;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const ELF: &[u8] = include_elf!("ev-exec-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version = clap::crate_version!(), about = "A CLI for running the ev-exec SP1 program", long_about = None)]
struct Args {
    #[arg(long, help = "Run the program in execute mode")]
    execute: bool,

    #[arg(long, help = "Run the program in prove mode")]
    prove: bool,

    #[arg(long, help = "The Celestia block height")]
    height: u64,

    #[arg(long, help = "Output file for benchmark report in JSON format")]
    output_file: Option<String>,

    #[arg(long, help = "Trusted EVM height which contains trusted state root")]
    trusted_height: Option<u64>,

    #[arg(long, help = "Trusted state root (hex string) for the trusted height")]
    trusted_root: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BenchmarkReport {
    pub total_blobs: u64,
    pub total_blockexec_inputs: u64,
    pub total_tx_count: u64,
    pub total_evm_gas: u64,
    pub total_gas: u64,
    pub total_instruction_count: u64,
    pub total_syscall_count: u64,
    pub cycle_tracker_results: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProofInputMetrics {
    pub total_blobs: u64,
    pub total_blockexec_inputs: u64,
    pub total_tx_count: u64,
    pub total_evm_gas: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    sp1_sdk::utils::setup_logger();
    dotenvy::dotenv().ok();

    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    if args.height == 0 {
        eprintln!("Error: You must specify a block number using --height");
        std::process::exit(1);
    }

    let height = args.height;
    let input_dir = format!("testdata/inputs/block-{height}");

    let client = ProverClient::from_env();

    let mut stdin = SP1Stdin::new();
    let proof_input_metrics = write_proof_inputs(&mut stdin, &input_dir, &args)?;

    if args.execute {
        // Execute the program.
        let (output, report) = client.execute(ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");

        // Read the output.
        let block_exec_output: BlockExecOutput = bincode::deserialize(output.as_slice())?;
        println!("Outputs: {block_exec_output}");

        // Record the total gas and number of cycles executed.
        println!("Total gas: {}", report.gas.unwrap());
        println!("Total instruction count: {}", report.total_instruction_count());
        println!("Total syscall count: {}", report.total_syscall_count());

        // If an output opt is provided then write the results to JSON.
        if let Some(output_file) = args.output_file {
            let benchmark_report = BenchmarkReport {
                total_blobs: proof_input_metrics.total_blobs,
                total_blockexec_inputs: proof_input_metrics.total_blockexec_inputs,
                total_tx_count: proof_input_metrics.total_tx_count,
                total_evm_gas: proof_input_metrics.total_evm_gas,
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
        let (pk, vk) = client.setup(ELF);

        // Generate the proof.
        let proof = client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Save the proof and reload.
        let proof_path = format!("testdata/proofs/proof-with-pis-{height}.bin");
        proof.save(&proof_path)?;
        let deserialized_proof = SP1ProofWithPublicValues::load(&proof_path)?;

        // Verify the proof.
        client.verify(&deserialized_proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }

    Ok(())
}

fn write_proof_inputs(stdin: &mut SP1Stdin, input_dir: &str, args: &Args) -> Result<ProofInputMetrics, Box<dyn Error>> {
    let header_json = fs::read_to_string(format!("{input_dir}/header.json"))?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;

    let dah_json = fs::read_to_string(format!("{input_dir}/dah.json"))?;
    let dah: DataAvailabilityHeader = serde_json::from_str(&dah_json)?;

    let blobs_json = fs::read_to_string(format!("{input_dir}/blobs.json"))?;
    let blobs: Vec<Blob> = serde_json::from_str(&blobs_json)?;
    let blobs_raw = serde_cbor::to_vec(&blobs)?;

    let pub_key_encoded = fs::read(format!("{input_dir}/pub_key.bin"))?;
    let pub_key = bincode::deserialize(&pub_key_encoded)?;

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE env variable must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let proofs_encoded = fs::read(format!("{input_dir}/namespace_proofs.bin"))?;
    let proofs: Vec<NamespaceProof> = bincode::deserialize(&proofs_encoded)?;

    let executor_inputs_encoded = fs::read(format!("{input_dir}/executor_inputs.bin"))?;
    let executor_inputs: Vec<EthClientExecutorInput> = bincode::deserialize(&executor_inputs_encoded)?;

    // Determine trusted height
    let trusted_height = if let Some(h) = args.trusted_height {
        h
    } else if let Some(input) = executor_inputs.first() {
        input.parent_header().number
    } else {
        panic!("Trusted height not provided and executor_inputs is empty");
    };

    // Determine trusted root
    let trusted_root = if let Some(root_str) = args.trusted_root.as_deref() {
        let bytes = hex::decode(root_str).expect("Invalid hex");
        let array: [u8; 32] = bytes.try_into().expect("Trusted root must be 32 bytes");
        FixedBytes::from(array)
    } else if let Some(input) = executor_inputs.first() {
        input.state_anchor()
    } else {
        panic!("Trusted root not provided and executor_inputs is empty");
    };

    let total_tx_count: usize = executor_inputs
        .iter()
        .map(|input| input.current_block.body.transactions.len())
        .sum();

    let total_gas: u64 = executor_inputs.iter().map(|input| input.current_block.gas_used).sum();

    let input = BlockExecInput {
        header_raw,
        dah,
        blobs_raw,
        pub_key,
        namespace,
        proofs,
        executor_inputs: executor_inputs.clone(),
        trusted_height,
        trusted_root,
    };
    stdin.write(&input);

    let proof_input_metrics = ProofInputMetrics {
        total_blobs: blobs.len() as u64,
        total_blockexec_inputs: executor_inputs.len() as u64,
        total_tx_count: total_tx_count as u64,
        total_evm_gas: total_gas,
    };

    Ok(proof_input_metrics)
}
