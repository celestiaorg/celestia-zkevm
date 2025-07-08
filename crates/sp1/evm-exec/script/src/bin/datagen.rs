//! A script that generates proof input data for a range of EVM blocks.
//!
//! This tool connects to an EVM reth node, sequencer and a Celestia data availability node, then collects and prepares:
//! - STF input (client execution input)
//! - Blob inclusion proof input
//! - Celestia header JSON
//!
//! For each Celestia block, the generated inputs are written to a directory structure:
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
use celestia_types::nmt::Namespace;
use celestia_types::{Blob, ExtendedHeader, ShareProof};
use clap::Parser;
use eyre::Context;
use prost::Message;
use reth_chainspec::ChainSpec;
use rollkit_types::v1::store_service_client::StoreServiceClient;
use rollkit_types::v1::{GetMetadataRequest, SignedData, SignedHeader};
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;

mod config {
    pub const CELESTIA_RPC_URL: &str = "http://localhost:26658";
    pub const EVM_RPC_URL: &str = "http://localhost:8545";
    pub const ROLLKIT_URL: &str = "http://localhost:7331";
}

/// The sentinel data hash for empty transactions in sequencer SignedHeaders
const DATA_HASH_FOR_EMPTY_TXS: [u8; 32] = [
    110, 52, 11, 156, 255, 179, 122, 152, 156, 165, 68, 230, 187, 120, 10, 44, 120, 144, 29, 63, 179, 55, 56, 118, 133,
    17, 163, 6, 23, 175, 160, 29,
];

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

/// Queries the tx data Celestia inclusion height for an EVM block.
async fn data_inclusion_height(sequencer_rpc: String, block_number: u64) -> Result<u64> {
    let mut client = StoreServiceClient::connect(sequencer_rpc.clone()).await?;
    let req = GetMetadataRequest {
        key: format!("rhb/{}/d", block_number),
    };

    let resp = client.get_metadata(req).await?;
    let height = u64::from_le_bytes(resp.into_inner().value[..8].try_into()?);

    Ok(height)
}

/// Queries the Celestia data availability node for a ShareProof for the provided blob.
pub async fn blob_share_proof(client: &Client, header: &ExtendedHeader, blob: &Blob) -> Result<ShareProof> {
    let eds_size = header.dah.row_roots().len() as u64;
    let ods_size = eds_size / 2;
    let first_row_index = blob.index.unwrap() / eds_size;
    let ods_index = blob.index.unwrap() - (first_row_index * ods_size);

    // NOTE: mutated the app version in the header as otherwise we run into v4 unsupported issues
    let mut modified_header = header.clone();
    modified_header.header.version.app = 3;

    let range_response = client
        .share_get_range(&modified_header, ods_index, ods_index + blob.shares_len() as u64)
        .await?;

    let share_proof = range_response.proof;
    share_proof.verify(modified_header.dah.hash())?;

    Ok(share_proof)
}
fn write_proof_inputs(
    client_executor_input: EthClientExecutorInput,
    header: &ExtendedHeader,
    share_proof: Option<ShareProof>,
) -> Result<()> {
    // Create output dir: testdata/inputs/block-{celestia_block_number}/
    let block_dir = format!("testdata/inputs/block-{}", header.height());
    fs::create_dir_all(&block_dir)?;

    let json = serde_json::to_string_pretty(&header.header)?;
    fs::write(format!("{}/header.json", block_dir), json)?;

    let evm_block_number = client_executor_input.current_block.number;
    let encoded_input = bincode::serialize(&client_executor_input)?;
    fs::write(
        format!("{}/client_input-{}.bin", block_dir, evm_block_number),
        encoded_input,
    )?;

    if let Some(share_proof) = share_proof {
        let encoded_proof = bincode::serialize(&share_proof)?;
        fs::write(
            format!("{}/share_proof-{}.bin", block_dir, evm_block_number),
            encoded_proof,
        )?;
    }

    Ok(())
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

        let blobs = celestia_client.blob_get_all(block_number, &[namespace]).await?.unwrap();
        for blob in blobs {
            let header = match SignedHeader::decode(blob.data.as_slice()) {
                Ok(data) => data.header.unwrap(),
                Err(_) => continue,
            };

            println!(
                "Got SignedHeader {} at Celestia height: {}",
                header.height, block_number
            );

            let client_executor_input =
                generate_client_executor_input(config::EVM_RPC_URL, header.height, chain_spec.clone(), genesis.clone())
                    .await?;

            if header.data_hash != DATA_HASH_FOR_EMPTY_TXS {
                let data_height = data_inclusion_height(config::ROLLKIT_URL.to_string(), header.height).await?;

                let data_blobs = celestia_client.blob_get_all(data_height, &[namespace]).await?.unwrap();
                for data_blob in data_blobs {
                    let tx_data = match SignedData::decode(data_blob.data.as_slice()) {
                        Ok(data) => data.data.unwrap(),
                        Err(_) => continue,
                    };

                    if tx_data.metadata.unwrap().height == header.height {
                        println!(
                            "Got SignedData for Header {} at Celestia height: {}",
                            header.height, data_height
                        );

                        let extended_header = celestia_client.header_get_by_height(data_height).await?;
                        let share_proof = blob_share_proof(&celestia_client, &extended_header, &data_blob).await?;

                        write_proof_inputs(client_executor_input.clone(), &extended_header, Some(share_proof))?;
                    }
                }
            } else {
                let extended_header = celestia_client.header_get_by_height(block_number).await?;
                write_proof_inputs(client_executor_input, &extended_header, None)?;
            }
        }

        println!("Finished processing blobs for Celestia block: {}", block_number);
    }

    Ok(())
}
