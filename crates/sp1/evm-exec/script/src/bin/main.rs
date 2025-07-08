//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! The program loads input files from a block-specific directory (e.g., `testdata/inputs/block-1010/`)
//! and writes the resulting proof to `testdata/proofs/proof-with-pis-<height>.bin`.
//!
//! You must provide the block number via `--height`, along with either `--execute` or `--prove`.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 1010
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --prove --height 1010
//! ```
use std::error::Error;
use std::fs;

use anyhow::{Context, Result};
use celestia_types::ShareProof;
use clap::Parser;
use evm_exec_types::EvmBlockExecOutput;
use regex::Regex;
use rsp_client_executor::io::EthClientExecutorInput;
use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use tendermint::block::header::Header;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    height: u64,
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
    write_proof_inputs(&mut stdin, &input_dir)?;

    if args.execute {
        // Execute the program.
        let (output, report) = client.execute(EVM_EXEC_ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");

        // Read the output.
        let block_exec_output: EvmBlockExecOutput = bincode::deserialize(output.as_slice())?;
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

fn write_proof_inputs(stdin: &mut SP1Stdin, input_dir: &str) -> Result<(), Box<dyn Error>> {
    let inputs = read_client_inputs(input_dir)?;
    stdin.write(&inputs);

    let header_json = fs::read_to_string(format!("{input_dir}/header.json"))?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;
    stdin.write_vec(header_raw);

    let share_proofs = read_share_proofs(input_dir)?;
    stdin.write(&share_proofs);

    Ok(())
}

/// Reads and deserializes ordered `EthClientExecutorInput` files from the given directory.
fn read_client_inputs(input_dir: &str) -> Result<Vec<EthClientExecutorInput>> {
    let pattern = Regex::new(r"^client_input-(\d+)\.bin$")?;

    let mut indexed_paths: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let caps = pattern.captures(file_name)?;
            let index = caps.get(1)?.as_str().parse::<usize>().ok()?;
            Some((index, path))
        })
        .collect();

    indexed_paths.sort_by_key(|(index, _)| *index);

    indexed_paths
        .into_iter()
        .map(|(_, path)| {
            let bytes = fs::read(&path).with_context(|| format!("reading file {:?}", path))?;
            bincode::deserialize::<EthClientExecutorInput>(&bytes)
                .with_context(|| format!("deserializing file {:?}", path))
        })
        .collect()
}

/// Reads and deserializes ordered `ShareProof` files from the given directory.
fn read_share_proofs(input_dir: &str) -> Result<Vec<ShareProof>> {
    let pattern = Regex::new(r"^share_proof-(\d+)\.bin$")?;

    let mut indexed_paths: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let caps = pattern.captures(file_name)?;
            let index = caps.get(1)?.as_str().parse::<u32>().ok()?;
            Some((index, path))
        })
        .collect();

    indexed_paths.sort_by_key(|(index, _)| *index);

    indexed_paths
        .into_iter()
        .map(|(_, path)| {
            let bytes = fs::read(&path).with_context(|| format!("reading file {:?}", path))?;
            bincode::deserialize::<ShareProof>(&bytes).with_context(|| format!("deserializing file {:?}", path))
        })
        .collect()
}
