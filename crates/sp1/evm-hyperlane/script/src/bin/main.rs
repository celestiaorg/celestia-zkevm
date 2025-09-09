//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run -p evm-hyperlane-script --release -- --execute --contract 0xFCb1d485ef46344029D9E8A7925925e146B3430E --start-idx 0 --end-idx 23 --target-height 268 --rpc-url http://127.0.0.1:8545
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run -p evm-hyperlane-script --release -- --prove --contract 0xFCb1d485ef46344029D9E8A7925925e146B3430E --start-idx 0 --end-idx 23 --target-height 268 --rpc-url http://127.0.0.1:8545
//! ```

use std::{str::FromStr, time::Instant};

use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use clap::{Parser, command};
use evm_hyperlane_types_sp1::{HyperlaneMessageInputs, HyperlaneMessageOutputs, tree::MerkleTree};
use evm_storage_proofs::{
    client::EvmClient,
    types::{HYPERLANE_MERKLE_TREE_KEYS, HyperlaneBranchProof, HyperlaneBranchProofInputs},
};
use sp1_sdk::{ProverClient, SP1Stdin, include_elf};
use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};
use url::Url;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

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
    let message_db = HyperlaneMessageStore::from_env().unwrap();
    let mut messages = Vec::new();
    for height in args.start_idx..=args.end_idx {
        let message = message_db.get_message(height).unwrap();
        messages.push(message);
    }

    let provider = ProviderBuilder::new().connect_http(Url::from_str(&args.rpc_url).unwrap());
    let evm_client = EvmClient::new(provider);
    let proof = evm_client
        .get_proof(
            &HYPERLANE_MERKLE_TREE_KEYS,
            Address::from_str(&args.contract).unwrap(),
            args.target_height.into(),
        )
        .await
        .unwrap();

    let execution_state_root = evm_client.get_state_root(args.target_height.into()).await.unwrap();
    let branch_proof = HyperlaneBranchProof::new(proof);

    let inputs = HyperlaneMessageInputs::new(
        execution_state_root,
        args.contract,
        messages.into_iter().map(|m| m.message).collect(),
        HyperlaneBranchProofInputs::from(branch_proof),
        MerkleTree::default(),
    );
    stdin.write(&inputs);

    if args.execute {
        client.execute(EVM_HYPERLANE_ELF, &stdin).run().unwrap();
        println!("Program executed successfully!");
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(EVM_HYPERLANE_ELF);
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
