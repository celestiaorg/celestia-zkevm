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
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 12 --trusted-height 18
//! --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --prove --height 12 --trusted-height 18
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
use evm_exec_types::BlockExecOutput;
use rsp_client_executor::io::{EthClientExecutorInput, WitnessInput};
use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use tendermint::block::header::Header;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version = clap::crate_version!(), about = "A CLI for running the evm-exec SP1 program", long_about = None)]
struct Args {
    #[arg(long, help = "Run the program in execute mode")]
    execute: bool,

    #[arg(long, help = "Run the program in prove mode")]
    prove: bool,

    #[arg(long, help = "The Celestia block height")]
    height: u64,

    #[arg(long, help = "Trusted EVM height which contains trusted state root")]
    trusted_height: Option<u64>,

    #[arg(long, help = "Trusted state root (hex string) for the trusted height")]
    trusted_root: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

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
    write_proof_inputs(&mut stdin, &input_dir, &args)?;

    if args.execute {
        // Execute the program.
        let (output, report) = client.execute(EVM_EXEC_ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");

        // Read the output.
        let block_exec_output: BlockExecOutput = bincode::deserialize(output.as_slice())?;
        println!("Outputs: {}", block_exec_output);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(EVM_EXEC_ELF);

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

fn write_proof_inputs(stdin: &mut SP1Stdin, input_dir: &str, args: &Args) -> Result<(), Box<dyn Error>> {
    let header_json = fs::read_to_string(format!("{input_dir}/header.json"))?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;
    stdin.write_vec(header_raw);

    let dah_json = fs::read_to_string(format!("{input_dir}/dah.json"))?;
    let dah: DataAvailabilityHeader = serde_json::from_str(&dah_json)?;
    stdin.write(&dah);

    let blobs_json = fs::read_to_string(format!("{input_dir}/blobs.json"))?;
    let blobs: Vec<Blob> = serde_json::from_str(&blobs_json)?;
    let blobs_raw = serde_cbor::to_vec(&blobs)?;
    stdin.write_vec(blobs_raw);

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE env variable must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    stdin.write(&namespace);

    let proofs_encoded = fs::read(format!("{input_dir}/namespace_proofs.bin"))?;
    let proofs: Vec<NamespaceProof> = bincode::deserialize(&proofs_encoded)?;
    stdin.write(&proofs);

    let executor_inputs_encoded = fs::read(format!("{input_dir}/executor_inputs.bin"))?;
    let executor_inputs: Vec<EthClientExecutorInput> = bincode::deserialize(&executor_inputs_encoded)?;
    stdin.write(&executor_inputs);

    // Determine trusted height
    let trusted_height = if let Some(h) = args.trusted_height {
        h
    } else if let Some(input) = executor_inputs.first() {
        input.parent_header().number
    } else {
        panic!("Trusted height not provided and executor_inputs is empty");
    };
    stdin.write(&trusted_height);

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
    stdin.write(&trusted_root);

    Ok(())
}
