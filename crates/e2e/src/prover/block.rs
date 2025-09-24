// This endpoint generates a block proof for a range (trusted_height, target_height)
// and wraps it recursively into a single groth16 proof using the ev-range-exec program.

use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use alloy_primitives::FixedBytes;
use ev_prover::config::config::Config;

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_provider::ProviderBuilder;
use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::Blob;
use celestia_types::nmt::{Namespace, NamespaceProof};
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::{GetBlockRequest, SignedData};
use eyre::Context;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;

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
    let mut sequencer_client = StoreServiceClient::connect(config::SEQUENCER_URL).await?;
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };

    let resp = sequencer_client.get_block(block_req).await?;
    let pub_key = resp.into_inner().block.unwrap().header.unwrap().signer.unwrap().pub_key;

    Ok(pub_key[4..].to_vec())
}

pub async fn prove_blocks(
    start_height: u64,
    num_blocks: u64,
    trusted_root: FixedBytes<32>,
    target_height: u64,
) -> Result<()> {
    dotenvy::dotenv().ok();
    let prover_mode = env::var("SP1_PROVER").unwrap_or("mock".to_string());
    // parallel mode (network)
    if prover_mode == "network" {
        parallel_prover().await?;
    }
    // synchroneous mode (cuda, cpu, mock)
    else {
        synchronous_prover(start_height, num_blocks).await?;
    }
    Ok(())
}

pub async fn parallel_prover() -> Result<(), Box<dyn Error>> {
    todo!("Implement parallel prover");
    Ok(())
}

pub async fn synchronous_prover(start_height: u64, num_blocks: u64) -> Result<()> {
    let genesis_path = env::var("GENESIS_PATH").expect("GENESIS_PATH must be set");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(&genesis_path)?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    let celestia_client = Client::new(config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;
    let pub_key = get_sequencer_pubkey().await?;

    // loop and adjsut inputs for each iteration,
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
            if row.proof.is_of_presence() {
                proofs.push(row.proof);
            }
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

        println!("Got EthClientExecutorInputs, total: {}", executor_inputs.len());
    }

    Ok(())
}
