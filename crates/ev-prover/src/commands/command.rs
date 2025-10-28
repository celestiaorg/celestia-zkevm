use std::{env, fs};

use crate::get_sequencer_pubkey;
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::{bail, Result};
use celestia_grpc_client::proto::celestia::zkism::v1::MsgCreateZkExecutionIsm;
use celestia_grpc_client::proto::hyperlane::warp::v1::MsgSetToken;
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::CelestiaIsmClient;
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::nmt::Namespace;
use celestia_types::{Blob, ExtendedHeader};
use ev_types::v1::SignedData;
use prost::Message;
use tracing::info;

use crate::commands::cli::VERSION;
use crate::config::config::{Config, APP_HOME, CONFIG_DIR, CONFIG_FILE, DEFAULT_GENESIS_JSON, GENESIS_FILE};
use crate::server::start_server;

pub fn init() -> Result<()> {
    let home_dir = dirs::home_dir().expect("cannot find home directory").join(APP_HOME);

    if !home_dir.exists() {
        info!("creating home directory at {home_dir:?}");
        fs::create_dir_all(&home_dir)?;
    }

    let config_dir = home_dir.join(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        info!("creating default config at {config_path:?}");
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config)?;
        fs::write(config_path, yaml)?;
    } else {
        info!("config file already exists at {config_path:?}");
    }

    let genesis_path = config_dir.join(GENESIS_FILE);
    if !genesis_path.exists() {
        info!("writing embedded genesis to {genesis_path:?}");
        fs::write(&genesis_path, DEFAULT_GENESIS_JSON)?;
    }

    Ok(())
}

pub async fn start() -> Result<()> {
    let config_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join(CONFIG_DIR)
        .join(CONFIG_FILE);

    if !config_path.exists() {
        bail!("config file not found at {}", config_path.display());
    }

    info!("reading config file at {}", config_path.display());
    let config_yaml = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&config_yaml)?;

    info!("starting gRPC server at {}", config.grpc_address);
    start_server(config).await?;

    Ok(())
}

pub async fn create_zkism() -> Result<()> {
    let celestia_rpc_url = env::var("CELESTIA_RPC_URL")?;
    let reth_rpc_url = env::var("RETH_RPC_URL")?;
    let sequencer_rpc_url = env::var("SEQUENCER_RPC_URL")?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE")?;
    let config = ClientConfig::from_env()?;
    let ism_client = CelestiaIsmClient::new(config).await?;
    let celestia_client = Client::new(&celestia_rpc_url, None).await?;
    let namespace: Namespace = Namespace::new_v0(&hex::decode(namespace_hex)?).unwrap();

    // Find a Celestia height with at least one blob (brute force backwards starting from head)
    let (celestia_state, blobs) = brute_force_head(&celestia_client, namespace).await?;
    // DA HEIGHT
    let height: u64 = celestia_state.height().value();
    // DA BLOCK HASH
    let block_hash = celestia_state.hash().as_bytes().to_vec();
    let last_blob = blobs.last().expect("User Error: Can't use a 0-blob checkpoint");
    let data = SignedData::decode(last_blob.data.as_slice())?;

    // EV BLOCK HEIGHT
    let last_blob_height = data.data.unwrap().metadata.unwrap().height;

    let provider = ProviderBuilder::new().connect_http(reth_rpc_url.parse()?);

    let block = provider
        .get_block(alloy_rpc_types::BlockId::Number(
            alloy_rpc_types::BlockNumberOrTag::Number(last_blob_height),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;

    // EV STATE ROOT
    let last_blob_state_root = block.header.state_root;
    // todo: deploy the ISM and Update
    let pub_key = get_sequencer_pubkey(sequencer_rpc_url).await?;

    let ev_hyperlane_vkey = fs::read("testdata/vkeys/ev-hyperlane-vkey-hash")?;
    let ev_combined_vkey = fs::read("testdata/vkeys/ev-range-exec-vkey-hash")?;
    let groth16_vkey = fs::read("testdata/vkeys/groth16_vk.bin")?;

    let create_message = MsgCreateZkExecutionIsm {
        creator: ism_client.signer_address().to_string(),
        state_root: last_blob_state_root.to_vec(),
        height: last_blob_height,
        celestia_header_hash: block_hash,
        celestia_height: height,
        namespace: namespace.as_bytes().to_vec(),
        sequencer_public_key: pub_key,
        groth16_vkey,
        state_transition_vkey: hex::decode(&ev_combined_vkey[2..]).unwrap(),
        state_membership_vkey: hex::decode(&ev_hyperlane_vkey[2..]).unwrap(),
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
    println!("version: {VERSION}");
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
                        info!("No blobs at height {}, trying nexst height", current_height);
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
