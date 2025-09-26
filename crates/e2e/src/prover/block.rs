// This endpoint generates a block proof for a range (trusted_height, target_height)
// and wraps it recursively into a single groth16 proof using the ev-range-exec program.

use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_primitives::{FixedBytes, hex};
use alloy_provider::ProviderBuilder;
use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::Blob;
use celestia_types::nmt::{Namespace, NamespaceProof};
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::{GetBlockRequest, SignedData};
use ev_zkevm_types::programs::block::{BlockExecInput, BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput};
use eyre::Context;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use sp1_sdk::{HashableKey, SP1Proof, SP1Stdin};
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues};

mod config {
    pub const CELESTIA_RPC_URL: &str = "http://localhost:26658";
    pub const EVM_RPC_URL: &str = "http://localhost:8545";
    pub const SEQUENCER_URL: &str = "http://localhost:7331";
}

/// Loads the genesis file from disk and converts it into a ChainSpec
fn load_chain_spec_from_genesis(path: &str) -> Result<(Genesis, Arc<ChainSpec>), Box<dyn Error>> {
    let genesis_json = fs::read_to_string(path).wrap_err_with(|| format!("Failed to read genesis file at {path}"))?;
    let alloy_genesis: AlloyGenesis = serde_json::from_str(&genesis_json)?;

    let genesis = Genesis::Custom(alloy_genesis.config);
    let chain_spec: Arc<ChainSpec> = Arc::new((&genesis).try_into()?);

    Ok((genesis, chain_spec))
}

/// Generates the client executor input (STF) for an EVM block.
async fn generate_client_executor_input(
    rpc_url: &str,
    block_number: u64,
    chain_spec: Arc<ChainSpec>,
    genesis: Genesis,
) -> Result<EthClientExecutorInput, Box<dyn Error>> {
    let host_executor = EthHostExecutor::eth(chain_spec.clone(), None);

    let provider = ProviderBuilder::new().connect_http(rpc_url.parse().unwrap());
    let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

    let client_input = host_executor
        .execute(block_number, &rpc_db, &provider, genesis, None, false)
        .await
        .wrap_err_with(|| format!("Failed to execute block {block_number}"))?;

    Ok(client_input)
}

async fn get_sequencer_pubkey() -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Connecting to sequencer url: {}", config::SEQUENCER_URL);
    let mut sequencer_client = StoreServiceClient::connect(config::SEQUENCER_URL).await?;
    println!("Connected to sequencer url: {}", config::SEQUENCER_URL);
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };
    println!("Getting block from sequencer url: {}", config::SEQUENCER_URL);
    let resp = sequencer_client.get_block(block_req).await?;
    println!("Got block from sequencer url: {}", config::SEQUENCER_URL);
    let pub_key = resp.into_inner().block.unwrap().header.unwrap().signer.unwrap().pub_key;

    Ok(pub_key[4..].to_vec())
}

pub async fn prove_blocks(
    start_height: u64,
    trusted_height: u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
) -> Result<SP1ProofWithPublicValues, Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let mut trusted_height = trusted_height;
    let prover_mode = env::var("SP1_PROVER").unwrap_or("mock".to_string());
    // parallel mode (network)
    let proof = {
        if prover_mode == "network" {
            panic!("Parallel prover is not implemented");
        }
        // synchroneous mode (cuda, cpu, mock)
        else {
            synchroneous_prover(start_height, &mut trusted_height, num_blocks, trusted_root).await?
        }
    };
    Ok(proof)
}

pub async fn parallel_prover() -> Result<(), Box<dyn Error>> {
    todo!("Implement parallel prover");
}

pub async fn synchroneous_prover(
    start_height: u64,
    trusted_height: &mut u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
) -> Result<SP1ProofWithPublicValues, Box<dyn Error>> {
    let genesis_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("config")
        .join("genesis.json");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(genesis_path.to_str().unwrap())?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    let celestia_client = Client::new(config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;
    let pub_key = get_sequencer_pubkey().await?;
    let client = ProverClient::from_env();
    let block_prover_elf = fs::read("elfs/ev-exec-elf").expect("Failed to read ELF");
    let (pk, vk) = client.setup(&block_prover_elf);

    let mut block_proofs: Vec<SP1ProofWithPublicValues> = Vec::new();
    // loop and adjust inputs for each iteration,
    // collect all proofs into a vec and return
    // later wrap them in g16
    for block_number in start_height..(start_height + num_blocks) {
        println!("\nProcessing block: {block_number}");
        let blobs: Vec<Blob> = celestia_client
            .blob_get_all(block_number, &[namespace])
            .await?
            .unwrap_or_default();
        println!("Got {} blobs for block: {}", blobs.len(), block_number);

        let extended_header = celestia_client.header_get_by_height(block_number).await?;
        let namespace_data = celestia_client
            .share_get_namespace_data(&extended_header, namespace)
            .await?;
        let mut proofs: Vec<NamespaceProof> = Vec::new();
        for row in namespace_data.rows {
            proofs.push(row.proof);
        }
        println!("Got NamespaceProofs, total: {}", proofs.len());

        let mut executor_inputs: Vec<EthClientExecutorInput> = Vec::new();
        for blob in blobs.as_slice() {
            let data = match SignedData::decode(blob.data.as_slice()) {
                Ok(data) => data.data.unwrap(),
                Err(_) => continue,
            };
            let height = data.metadata.unwrap().height;
            println!("Got SignedData for EVM block {height}");

            let client_executor_input =
                generate_client_executor_input(config::EVM_RPC_URL, height, chain_spec.clone(), genesis.clone())
                    .await?;
            executor_inputs.push(client_executor_input);
        }

        let mut stdin = SP1Stdin::new();
        let input = BlockExecInput {
            header_raw: serde_cbor::to_vec(&extended_header.header)?,
            dah: extended_header.dah,
            blobs_raw: serde_cbor::to_vec(&blobs)?,
            pub_key: pub_key.clone(),
            namespace,
            proofs,
            executor_inputs: executor_inputs.clone(),
            trusted_height: *trusted_height,
            trusted_root: *trusted_root,
        };

        stdin.write(&input);
        println!("Generating proof for block: {block_number}, trusted height: {trusted_height}");
        let proof = client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .expect("failed to generate proof");
        block_proofs.push(proof.clone());
        println!("Proof generated successfully!");

        let public_values: BlockExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
        // update trusted root and height
        *trusted_root = public_values.new_state_root.into();
        *trusted_height = public_values.new_height;
        println!("New state root: {:?}", *trusted_root);
    }

    // reinitialize the prover client
    let client = ProverClient::from_env();
    let mut stdin = SP1Stdin::new();
    let range_prover_elf = fs::read("elfs/ev-range-exec-elf").expect("Failed to read ELF");
    let (pk, _) = client.setup(&range_prover_elf);

    let vkeys = vec![vk.hash_u32(); block_proofs.len()];

    let public_inputs = block_proofs
        .iter()
        .map(|proof| proof.public_values.to_vec())
        .collect::<Vec<_>>();

    let input = BlockRangeExecInput {
        vkeys,
        public_values: public_inputs,
    };
    stdin.write(&input);

    for block_proof in &block_proofs {
        let SP1Proof::Compressed(ref proof) = block_proof.proof else {
            panic!()
        };
        stdin.write_proof(*proof.clone(), vk.vk.clone());
    }

    let proof = client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");

    let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
    println!("Target state root: {:?}", public_values.new_state_root);

    Ok(proof)
}
