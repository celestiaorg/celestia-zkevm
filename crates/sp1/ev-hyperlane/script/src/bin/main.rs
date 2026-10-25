//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run -p ev-hyperlane-script --release -- --execute --contract 0x6007cE81D2FD7b9b7f22e71cE9896e00d6017ba8 --height 268 --rpc-url http://127.0.0.1:8545
//! ```

use alloy_primitives::{hex::FromHex, Address, FixedBytes};
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::{Context, Result};
use clap::Parser;
use ev_zkevm_types::hyperlane::{
    io::{HyperlaneTreeInputs, HyperlaneTreeOutputs},
    proof::{hyperlane_merkle_tree_state_keys, HyperlaneBranchProof, HyperlaneBranchProofInputs},
};
#[cfg(not(feature = "retry"))]
use sp1_sdk::ProvingKey;
use sp1_sdk::{include_elf, Elf, ProveRequest, Prover, ProverClient, SP1Stdin};
use std::{str::FromStr, time::Instant};
use url::Url;
#[cfg(feature = "retry")]
use {
    celestia_grpc_client::{
        types::ClientConfig, CelestiaIsmClient, MsgProcessMessage, MsgSubmitMessages, QueryIsmRequest,
    },
    ev_zkevm_types::{
        block::State,
        hyperlane::{
            encode_hyperlane_message,
            merkle::{encode_merkle_proof_metadata, MerkleTree},
        },
    },
    storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore},
};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_HYPERLANE_ELF: Elf = include_elf!("ev-hyperlane-program");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[cfg(not(feature = "retry"))]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    contract: String,

    /// The EVM block height whose state root the tree is proven against.
    #[arg(long)]
    height: u64,

    #[arg(long)]
    rpc_url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[cfg(feature = "retry")]
struct Args {
    #[arg(long)]
    contract: String,
    #[arg(long)]
    snapshot_index: u64,
    #[arg(long)]
    mailbox_id: String,
    #[arg(long)]
    rpc_url: String,
}

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");

    dotenvy::dotenv().ok();
    sp1_sdk::utils::setup_logger();
    let args = Args::parse();

    let client = ProverClient::from_env().await;

    #[cfg(not(feature = "retry"))]
    {
        if args.execute == args.prove {
            eprintln!("Error: You must specify either --execute or --prove");
            std::process::exit(1);
        }

        let mut stdin = SP1Stdin::new();
        write_proof_inputs(&mut stdin, &args.contract, &args.rpc_url, args.height)
            .await
            .expect("failed to write proof inputs");

        if args.execute {
            let (public_values, _) = client
                .execute(EV_HYPERLANE_ELF.clone(), stdin)
                .await
                .expect("failed to execute program");
            let outputs: HyperlaneTreeOutputs =
                bincode::deserialize(public_values.as_slice()).expect("Failed to deserialize program outputs");
            println!("Program executed successfully! Outputs: {outputs:?}");
        } else {
            let pk = client
                .setup(EV_HYPERLANE_ELF.clone())
                .await
                .expect("failed to setup prover");
            let vk = pk.verifying_key();
            let start_time = Instant::now();
            let proof = client.prove(&pk, stdin).await.expect("failed to generate proof");
            println!("Proof generation time: {:?}", Instant::now() - start_time);
            client.verify(&proof, vk, None).expect("failed to verify proof");
            println!("Successfully verified proof!");
            let proof_outputs: HyperlaneTreeOutputs =
                bincode::deserialize(proof.public_values.as_slice()).expect("Failed to deserialize proof outputs");
            println!("Proof outputs: {proof_outputs:?}");
        }
    }

    #[cfg(feature = "retry")]
    {
        let storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(".ev-prover")
            .join("data");
        let hyperlane_message_store = HyperlaneMessageStore::from_path(&storage_path).unwrap();
        let hyperlane_snapshot_store = HyperlaneSnapshotStore::from_path(&storage_path).unwrap();
        let previous_snapshot = hyperlane_snapshot_store
            .get_snapshot(args.snapshot_index)
            .expect("Fatal, snapshot was lost");

        let config = ClientConfig::from_env().unwrap();
        let ism_client = CelestiaIsmClient::new(config.clone()).await.unwrap();
        let ev_trusted_state = ism_client
            .ism(QueryIsmRequest {
                id: config.ism_id.clone(),
            })
            .await
            .unwrap();
        let state: State = bincode::deserialize(&ev_trusted_state.ism.unwrap().state).unwrap();
        let mut stdin = SP1Stdin::new();

        write_proof_inputs(&mut stdin, &args.contract, &args.rpc_url, state.height)
            .await
            .unwrap();

        let pk = client
            .setup(EV_HYPERLANE_ELF.clone())
            .await
            .expect("failed to setup prover");
        let start_time = Instant::now();

        let proof = client
            .prove(&pk, stdin)
            .groth16()
            .await
            .expect("failed to generate proof");
        println!("Proof generation time: {:?}", Instant::now() - start_time);

        let proof_msg = MsgSubmitMessages::new(
            config.ism_id,
            proof.bytes(),
            proof.public_values.as_slice().to_vec(),
            config.signer_address.clone(),
        );

        let response = ism_client.send_tx(proof_msg).await.unwrap();
        if !response.success {
            panic!("Failed to submit proof: {:?}", response);
        }

        // Every message is relayed with a merkle proof against the proven root, which needs all leaves of the tree.
        let leaves: Vec<String> = hyperlane_message_store
            .get_up_to_block(state.height)
            .expect("Failed to get messages")
            .into_iter()
            .map(|m| m.message.id())
            .collect();

        // Relay verified messages to Celestia
        let mut messages = Vec::new();
        for height in previous_snapshot.height..=state.height {
            let block_messages = hyperlane_message_store
                .get_by_block(height)
                .expect("Failed to get messages");
            for block_message in block_messages {
                messages.push(block_message);
            }
        }

        for message in messages {
            let index = message.message.nonce;
            let branch = MerkleTree::merkle_proof(&leaves, index as usize).expect("Failed to build merkle proof");
            let metadata = encode_merkle_proof_metadata(index, &branch).expect("Failed to encode metadata");
            let message_hex = alloy::hex::encode(encode_hyperlane_message(&message.message).unwrap());
            let msg = MsgProcessMessage::new(
                args.mailbox_id.clone(),
                config.signer_address.clone(),
                alloy::hex::encode(metadata),
                message_hex,
            );
            let response = match ism_client.send_tx(msg).await {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("Failed to relay message: {:?}", e);
                    continue;
                }
            };
            if !response.success {
                eprintln!("Failed to relay message: {:?}", response);
            } else {
                println!("Successfully relayed message: {:?}", message.message.id());
            }
        }
    }
}

/// Write the program inputs: a storage proof of the Hyperlane tree branch and count at the given height.
async fn write_proof_inputs(stdin: &mut SP1Stdin, contract: &str, rpc_url: &str, height: u64) -> Result<()> {
    let provider = ProviderBuilder::new().connect_http(Url::from_str(rpc_url).expect("Failed to create provider"));
    let block_id = alloy_eips::BlockId::Number(alloy_eips::BlockNumberOrTag::Number(height));
    let proof = provider
        .get_proof(
            Address::from_str(contract).expect("Failed to create contract address"),
            hyperlane_merkle_tree_state_keys()
                .iter()
                .map(|k| FixedBytes::from_hex(k).expect("Failed to create fixed bytes"))
                .collect(),
        )
        .block_id(block_id)
        .await
        .expect("Failed to get proof");
    let block = provider.get_block(block_id).await?.context("Failed to get block")?;
    let execution_state_root = alloy::hex::encode(block.header.state_root.0);
    let inputs = HyperlaneTreeInputs::new(
        execution_state_root,
        contract.to_string(),
        HyperlaneBranchProofInputs::from(HyperlaneBranchProof::new(proof)),
    );
    stdin.write(&inputs);
    Ok(())
}
