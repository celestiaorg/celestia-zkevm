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

use celestia_types::{blob::Blob, AppVersion, ShareProof};
use clap::Parser;
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::{simple_merkle::proof::Proof, TmSha2Hasher};
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
    let input_dir = format!("testdata/new_inputs/block-{height}");

    let client = ProverClient::from_env();

    let mut stdin = SP1Stdin::new();
    write_new_proof_inputs(&mut stdin, &input_dir)?;

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

/// write_proof_inputs writes the program inputs to the provided SP1Stdin read from the input directory.
fn write_proof_inputs(stdin: &mut SP1Stdin, input_dir: &str) -> Result<(), Box<dyn Error>> {
    let client_executor_input: EthClientExecutorInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/client_input.bin"))?)?;
    stdin.write(&client_executor_input);

    let header_json = fs::read_to_string(format!("{input_dir}/header.json"))?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;
    stdin.write_vec(header_raw);

    let blob_proof: KeccakInclusionToDataRootProofInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/blob_proof.bin"))?)?;
    stdin.write(&blob_proof);

    let data_root_proof: Proof<TmSha2Hasher> =
        bincode::deserialize(&fs::read(format!("{input_dir}/data_root_proof.bin"))?)?;
    stdin.write(&data_root_proof);

    Ok(())
}

fn write_new_proof_inputs(stdin: &mut SP1Stdin, input_dir: &str) -> Result<(), Box<dyn Error>> {
    // let mut count: u64 = 0;

    // let mut entries: Vec<_> = fs::read_dir(input_dir)?
    //     .filter_map(|e| {
    //         let e = e.ok()?;
    //         let path = e.path();
    //         let name = path.file_name()?.to_str()?.to_string();

    //         if name.starts_with("client_input-") {
    //             Some((extract_suffix_number(&name)?, path))
    //         } else {
    //             None
    //         }
    //     })
    //     .collect();

    // entries.sort_by_key(|(num, _)| *num);

    // for entry in entries {
    //     // let entry = entry?;
    //     let path = entry.1.to_path_buf();

    //     // Filter only files with names like "client_input-<number>"
    //     if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
    //         if filename.starts_with("client_input-") {
    //             count = count + 1;
    //             let client_executor_input: EthClientExecutorInput = bincode::deserialize(&fs::read(&path)?)?;
    //             stdin.write(&client_executor_input);
    //         }
    //     }
    // }
    // println!("wrote {} client inputs", count);
    let mut inputs = vec![];

    let client_executor_input: EthClientExecutorInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/client_input-45.bin"))?)?;

    inputs.push(client_executor_input);

    let client_executor_input2: EthClientExecutorInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/client_input-46.bin"))?)?;

    inputs.push(client_executor_input2);

    let client_executor_input3: EthClientExecutorInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/client_input-47.bin"))?)?;

    inputs.push(client_executor_input3);

    let client_executor_input4: EthClientExecutorInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/client_input-48.bin"))?)?;

    inputs.push(client_executor_input4);

    stdin.write(&inputs);

    let header_json = fs::read_to_string(format!("{input_dir}/header.json"))?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;
    stdin.write_vec(header_raw);

    let blob_proof: KeccakInclusionToDataRootProofInput =
        bincode::deserialize(&fs::read(format!("{input_dir}/blob_proof-48.bin"))?)?;

    let blob =
        Blob::new(blob_proof.namespace_id, blob_proof.data, AppVersion::V3).expect("failed to construct Celestia blob");

    let share_data = blob.to_shares().expect("failed to convert blob to shares");

    let mut share_proofs = vec![];

    let share_proof = ShareProof {
        data: share_data
            .into_iter()
            .map(|s| s.as_ref().try_into().expect("invalid share length"))
            .collect(),
        namespace_id: blob_proof.namespace_id,
        row_proof: blob_proof.row_proof,
        share_proofs: blob_proof.share_proofs,
    };

    share_proofs.push(share_proof);

    stdin.write(&share_proofs);

    Ok(())
}
