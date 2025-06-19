//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p evm-exec-script --release -- --prove
//! ```
use std::error::Error;
use std::fs;

use clap::Parser;
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::EvmBlockExecOutput;
use nmt_rs::{simple_merkle::proof::Proof, TmSha2Hasher};
use rsp_client_executor::io::EthClientExecutorInput;
use sp1_sdk::{include_elf, ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use tendermint::block::header::Header;
use tendermint_proto::Protobuf;

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
        let (output, report) = client.execute(EVM_EXEC_ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");

        // Read the output.
        let block_exec_output: EvmBlockExecOutput = bincode::deserialize(output.as_slice())?;
        println!("Outputs: {}", serde_json::to_string_pretty(&block_exec_output)?);

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
        proof.save("testdata/proofs/proof-with-pis.bin")?;
        let deserialized_proof = SP1ProofWithPublicValues::load("testdata/proofs/proof-with-pis.bin")?;

        // Verify the proof.
        client.verify(&deserialized_proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }

    Ok(())
}

/// write_proof_inputs writes the program inputs to provided SP1Stdin
fn write_proof_inputs(stdin: &mut SP1Stdin) -> Result<(), Box<dyn Error>> {
    let blob_proof_data = fs::read("testdata/blob_proof.bin")?;
    let blob_proof: KeccakInclusionToDataRootProofInput = bincode::deserialize(&blob_proof_data)?;
    stdin.write(&blob_proof);

    let client_input_data = fs::read("testdata/client_input.bin")?;
    let client_executor_input: EthClientExecutorInput = bincode::deserialize(&client_input_data)?;
    stdin.write(&client_executor_input);

    let header_json = fs::read_to_string("testdata/header.json")?;
    let header: Header = serde_json::from_str(&header_json)?;
    let header_raw = serde_cbor::to_vec(&header)?;
    stdin.write_vec(header_raw);

    let data_hash = header.data_hash.unwrap().encode_vec();
    stdin.write_vec(data_hash);

    let data_root_proof_data = fs::read("testdata/data_root_proof.bin")?;
    let data_root_proof: Proof<TmSha2Hasher> = bincode::deserialize(&data_root_proof_data)?;
    stdin.write(&data_root_proof);

    let trusted_state_root = client_executor_input.parent_state.state_root().as_slice().to_vec();
    stdin.write_vec(trusted_state_root);

    Ok(())
}
