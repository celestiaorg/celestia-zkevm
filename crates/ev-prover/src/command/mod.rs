use alloy_provider::{Provider, ProviderBuilder};
use anyhow::Result;
use celestia_grpc_client::proto::celestia::zkism::v1::MsgCreateZkExecutionIsm;
use celestia_grpc_client::proto::hyperlane::warp::v1::MsgSetToken;
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::CelestiaIsmClient;
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::nmt::Namespace;
use celestia_types::{Blob, ExtendedHeader};
use ev_types::v1::SignedData;
use prost::Message;
use sp1_sdk::{HashableKey, Prover, ProverClient};
use tracing::info;

use crate::commands::cli::VERSION;
use crate::config::Config;
use crate::get_sequencer_pubkey;
use crate::prover::programs::combined::EV_COMBINED_ELF;
use crate::prover::programs::message::EV_HYPERLANE_ELF;
use crate::server::start_server;
use storage::proofs::{ProofStorage, RocksDbProofStorage};

pub mod cli;
pub use cli::{Cli, Commands};

pub fn init() -> Result<()> {
    Config::init()?;

    Ok(())
}

pub async fn start() -> Result<()> {
    let config = Config::load()?;
    info!("starting gRPC server at {}", config.grpc_address);
    start_server(config).await?;

    Ok(())
}

pub fn unsafe_reset_db() -> Result<()> {
    let storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join("data")
        .join("proofs.db");

    info!("resetting db state at {}", storage_path.display());

    let mut storage = RocksDbProofStorage::new(storage_path)?;
    storage.unsafe_reset()?;
    Ok(())
}

pub async fn create_zkism() -> Result<()> {
    let config = Config::load()?;
    let ism_client = CelestiaIsmClient::new(ClientConfig::from_env()?).await?;

    let celestia_client = Client::new(&config.rpc.celestia_rpc, None).await?;
    let namespace = config.namespace;

    // Find a Celestia height with at least one blob (brute force backwards starting from head)
    let (header, blobs) = brute_force_head(&celestia_client, namespace).await?;
    // DA HEIGHT
    let height: u64 = header.height().value();
    // DA BLOCK HASH
    let block_hash = header.hash().as_bytes().to_vec();
    let last_blob = blobs.last().expect("User Error: Can't use a 0-blob checkpoint");
    let data = SignedData::decode(last_blob.data.as_slice())?;

    // EV BLOCK HEIGHT
    let last_blob_height = data.data.unwrap().metadata.unwrap().height;

    let provider = ProviderBuilder::new().connect_http(config.rpc.evreth_rpc.parse()?);

    let block = provider
        .get_block(alloy_rpc_types::BlockId::Number(
            alloy_rpc_types::BlockNumberOrTag::Number(last_blob_height),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;

    // EV STATE ROOT
    let last_blob_state_root = block.header.state_root;
    // todo: deploy the ISM and Update
    let pub_key = get_sequencer_pubkey(config.rpc.evnode_rpc).await?;

    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EV_COMBINED_ELF);
    let state_transition_vkey = vk.bytes32_raw().to_vec();

    let (_, vk) = prover.setup(EV_HYPERLANE_ELF);
    let state_membership_vkey = vk.bytes32_raw().to_vec();

    let groth16_vkey = Config::groth16_vkey();

    let create_message = MsgCreateZkExecutionIsm {
        creator: ism_client.signer_address().to_string(),
        state_root: last_blob_state_root.to_vec(),
        height: last_blob_height,
        celestia_header_hash: block_hash,
        celestia_height: height,
        namespace: namespace.as_bytes().to_vec(),
        sequencer_public_key: pub_key,
        groth16_vkey,
        state_transition_vkey,
        state_membership_vkey,
    };

    let response = ism_client.send_tx(create_message).await?;
    assert!(response.success);
    info!("ISM created successfully");
    Ok(())
}

pub async fn update_ism(ism_id: String, token_id: String) -> Result<()> {
    let config = ClientConfig::from_env()?;
    let ism_client = CelestiaIsmClient::new(config).await?;

    //todo update
    let message = MsgSetToken {
        owner: ism_client.signer_address().to_string(),
        token_id,
        new_owner: ism_client.signer_address().to_string(),
        ism_id,
        renounce_ownership: false,
    };

    ism_client.send_tx(message).await?;
    info!("ISM updated successfully");

    Ok(())
}

pub fn version() {
    info!("version: {VERSION}");
}

async fn brute_force_head(celestia_client: &Client, namespace: Namespace) -> Result<(ExtendedHeader, Vec<Blob>)> {
    // Find a Celestia height with at least one blob (brute force backwards starting from head)
    let mut search_height: u64 = celestia_client.header_local_head().await.unwrap().height().value();
    let (celestia_state, blobs) = loop {
        match celestia_client.header_get_by_height(search_height).await {
            Ok(state) => {
                let current_height = state.height().value();
                match celestia_client.blob_get_all(current_height, &[namespace]).await {
                    Ok(Some(blobs)) if !blobs.is_empty() => {
                        info!("Found {} blob(s) at Celestia height {}", blobs.len(), current_height);
                        break (state, blobs);
                    }
                    Ok(_) => {
                        info!("No blobs at height {}, trying next height", current_height);
                        search_height -= 1;
                    }
                    Err(e) => {
                        info!(
                            "Error fetching blobs at height {}: {}, trying next height",
                            current_height, e
                        );
                        search_height -= 1;
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to get header at height {search_height}: {e}"));
            }
        }
    };
    Ok((celestia_state, blobs))
}
