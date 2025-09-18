//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run -p ev-hyperlane-script --release -- --execute --contract 0xFCb1d485ef46344029D9E8A7925925e146B3430E --start-idx 0 --end-idx 23 --target-height 268 --rpc-url http://127.0.0.1:8545
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p ev-hyperlane-script --release -- --prove --contract 0xFCb1d485ef46344029D9E8A7925925e146B3430E --start-idx 0 --end-idx 23 --target-height 268 --rpc-url http://127.0.0.1:8545
//! ```

use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use anyhow::Result;
use clap::{command, Parser};
use ev_hyperlane_types::{tree::MerkleTree, HyperlaneMessageInputs, HyperlaneMessageOutputs};
use ev_storage_proofs::{
    client::EvmClient,
    types::{HyperlaneBranchProof, HyperlaneBranchProofInputs, HYPERLANE_MERKLE_TREE_KEYS},
};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use std::{env, str::FromStr, time::Instant};
use storage::hyperlane::message::{HyperlaneMessageStore, IndexMode};
use url::Url;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_HYPERLANE_ELF: &[u8] = include_elf!("ev-hyperlane-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    contract: String,

    #[arg(long)]
    start_idx: u32,

    #[arg(long)]
    end_idx: u32,

    #[arg(long)]
    target_height: u32,

    #[arg(long)]
    rpc_url: String,
}

#[tokio::main]
async fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    write_proof_inputs(&mut stdin, &args)
        .await
        .expect("failed to write proof inputs");

    if args.execute {
        client.execute(EV_HYPERLANE_ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(EV_HYPERLANE_ELF);
        let start_time = Instant::now();
        // Generate the proof
        let proof = client.prove(&pk, &stdin).run().expect("failed to generate proof");
        println!("Proof generation time: {:?}", Instant::now() - start_time);
        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");

        let proof_outputs: HyperlaneMessageOutputs = bincode::deserialize(proof.public_values.as_slice()).unwrap();
        println!("Proof outputs: {proof_outputs:?}");
    }
}

async fn write_proof_inputs(stdin: &mut SP1Stdin, args: &Args) -> Result<()> {
    let message_db = HyperlaneMessageStore::from_path_relative(4, IndexMode::Message).unwrap();
    let mut messages = Vec::new();
    // insert messages into local database
    for height in args.start_idx..=args.end_idx {
        let message = message_db.get_message(height as u64).unwrap();
        messages.push(message);
    }

    // get the merkle proofs from the EVM execution client
    let provider = ProviderBuilder::new().connect_http(Url::from_str(&args.rpc_url).unwrap());
    let evm_client = EvmClient::new(provider);
    let proof = evm_client
        .get_proof(
            &HYPERLANE_MERKLE_TREE_KEYS,
            Address::from_str(&args.contract).unwrap(),
            Some(args.target_height.into()),
        )
        .await
        .unwrap();

    let execution_state_root = evm_client.get_state_root(args.target_height.into()).await.unwrap();
    let branch_proof = HyperlaneBranchProof::new(proof);

    // write the inputs to the stdin
    let inputs = HyperlaneMessageInputs::new(
        execution_state_root,
        args.contract.clone(),
        messages.into_iter().map(|m| m.message).collect(),
        HyperlaneBranchProofInputs::from(branch_proof),
        MerkleTree::default(),
    );
    stdin.write(&inputs);
    Ok(())
}
