//! A script that generates proof input data for a range of Celestia blocks and their included EVM block data.
//!
//! This tool connects to a Celestia data availability node and EVM reth node, then collects and prepares:
//! - Celestia header and data availability header.
//! - Blobs for the provided namespace.
//! - NamespaceProofs for the provided namespace.
//! - EVM state transition inputs (EthClientExecutorInput) for n EVM blocks included in the Celestia block.
//!
//! For each Celestia block, the generated inputs are written to a directory:
//! `testdata/inputs/block-<number>`
//!
//! You can run this script using the following command from the root of this repository:
//! ```shell
//! cargo run -p evm-exec-script --bin data-gen --release -- --start <START_BLOCK> --blocks <NUM_BLOCKS>
//! ```
use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use alloy_provider::ProviderBuilder;
use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::nmt::{Namespace, NamespaceProof};
use celestia_types::Blob;
use clap::Parser;
use eyre::Context;
use prost::Message;
use reth_chainspec::ChainSpec;
use rollkit_types::v1::SignedData;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;

mod config {
    pub const CELESTIA_RPC_URL: &str = "http://localhost:26658";
    pub const EVM_RPC_URL: &str = "http://localhost:8545";
}

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    start: u64,

    #[arg(long)]
    blocks: u64,
}

/// Loads the genesis file from disk and converts it into a ChainSpec
fn load_chain_spec_from_genesis(path: &str) -> Result<(Genesis, Arc<ChainSpec>), Box<dyn Error>> {
    let genesis_json = fs::read_to_string(path).wrap_err_with(|| format!("Failed to read genesis file at {}", path))?;

    let genesis = Genesis::Custom(genesis_json);
    let chain_spec = Arc::new((&genesis).try_into()?);
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

    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

    let client_input = host_executor
        .execute(block_number, &rpc_db, &provider, genesis, None, false)
        .await
        .wrap_err_with(|| format!("Failed to execute block {}", block_number))?;

    Ok(client_input)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let num_blocks = args.blocks;
    let start_height = args.start;

    let genesis_path = env::var("GENESIS_PATH").expect("GENESIS_PATH must be set");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(&genesis_path)?;

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let celestia_client = Client::new(config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;

    for block_number in start_height..(start_height + num_blocks) {
        println!("\nProcessing block: {}", block_number);

        let blobs: Vec<Blob> = celestia_client.blob_get_all(block_number, &[namespace]).await?.unwrap();
        println!("Got {} blobs for block: {}", blobs.len(), block_number);

        let extended_header = celestia_client.header_get_by_height(block_number).await?;

        // Clone the Header such that we are not writing an incorrect header to testdata files
        // Required until celestia rust libs support v4
        let mut modified_header = extended_header.clone();
        modified_header.header.version.app = 3;

        let namespace_data = celestia_client
            .share_get_namespace_data(&modified_header, namespace)
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
            println!("Got SignedData for EVM block {}", height);

            let client_executor_input =
                generate_client_executor_input(config::EVM_RPC_URL, height, chain_spec.clone(), genesis.clone())
                    .await?;

            executor_inputs.push(client_executor_input);
        }

        // Create output dir: testdata/inputs/block-{celestia_block_number}/
        let block_dir = format!("testdata/inputs/block-{}", block_number);
        fs::create_dir_all(&block_dir)?;

        println!("Writing proof input data to: {}", block_dir);

        let header_json = serde_json::to_string_pretty(&extended_header.header)?;
        fs::write(format!("{}/header.json", block_dir), header_json)?;

        let dah_json = serde_json::to_string_pretty(&extended_header.dah)?;
        fs::write(format!("{}/dah.json", block_dir), dah_json)?;

        let blobs_encoded = serde_json::to_string_pretty(&blobs)?;
        fs::write(format!("{}/blobs.json", block_dir), blobs_encoded)?;

        let proofs_encoded = bincode::serialize(&proofs)?;
        fs::write(format!("{}/namespace_proofs.bin", block_dir), proofs_encoded)?;

        let executor_inputs_encoded = bincode::serialize(&executor_inputs)?;
        fs::write(format!("{}/executor_inputs.bin", block_dir), executor_inputs_encoded)?;

        println!("Finished processing blobs for Celestia block: {}", block_number);
    }

    Ok(())
}
