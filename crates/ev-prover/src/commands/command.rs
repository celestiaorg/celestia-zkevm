use std::fs;

use anyhow::{anyhow, bail, Result};
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{CelestiaIsmClient, MsgCreateZkExecutionIsm};
use celestia_rpc::{client::Client as CelestiaClient, HeaderClient};
use celestia_types::nmt::Namespace;
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::GetBlockRequest;
use sp1_sdk::{HashableKey, Prover, ProverClient};
use tonic::Request;
use tracing::info;

use crate::commands::cli::VERSION;
use crate::config::config::{
    Config, APP_HOME, CONFIG_DIR, CONFIG_FILE, DEFAULT_GENESIS_JSON, GENESIS_FILE, GROTH16_VK,
};
use crate::prover::programs::message::EV_HYPERLANE_ELF;
use crate::prover::programs::range::EV_RANGE_EXEC_ELF;
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

pub fn version() {
    println!("version: {VERSION}");
}

pub async fn create_ism() -> Result<()> {
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

    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;

    let mut store_client = StoreServiceClient::connect("http://127.0.0.1:7331").await?;
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };
    let block_resp = store_client.get_block(block_req).await?;
    let pub_key = block_resp
        .into_inner()
        .block
        .unwrap()
        .header
        .unwrap()
        .signer
        .unwrap()
        .pub_key;

    let sequencer_public_key = pub_key
        .get(4..)
        .ok_or_else(|| anyhow!("sequencer public key is too short"))?
        .to_vec();

    let state_resp = store_client.get_state(Request::new(())).await?;
    let (height, state_root, celestia_height) = match state_resp.into_inner().state {
        Some(state) => {
            info!(?state, "State from sequencer");

            (state.last_block_height, state.app_hash, state.da_height)
        }
        None => bail!("sequencer state response did not include state"),
    };

    if state_root.is_empty() {
        bail!("sequencer state returned an empty app hash");
    }

    if celestia_height == 0 {
        bail!("sequencer state returned zero DA height");
    }

    let raw_ns = hex::decode(config.namespace_hex)?;
    let namespace = Namespace::new_v0(raw_ns.as_ref())?;

    let celestia_rpc = format!("http://{}", config.celestia_rpc);
    let celestia_client = CelestiaClient::new(&celestia_rpc, None).await?;

    let celestia_header = celestia_client
        .header_get_by_height(celestia_height)
        .await
        .map_err(|e| anyhow!("failed to fetch celestia header at height {celestia_height}: {e}"))?;

    let celestia_header_hash = celestia_header.hash();
    let celestia_header_hash = celestia_header_hash.as_bytes().to_vec();

    let groth16_vkey = GROTH16_VK.to_vec();

    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EV_RANGE_EXEC_ELF);
    let state_transition_vkey = vk.hash_bytes().to_vec();

    let (_, vk) = prover.setup(EV_HYPERLANE_ELF);
    let state_membership_vkey = vk.hash_bytes().to_vec();

    let signer_address = client.signer_address().to_string();
    let create_ism_msg = MsgCreateZkExecutionIsm::new(
        signer_address,
        state_root,
        height,
        celestia_header_hash,
        celestia_height,
        namespace.as_bytes().to_vec(),
        sequencer_public_key,
        groth16_vkey,
        state_transition_vkey,
        state_membership_vkey,
    );

    let res = client.send_tx(create_ism_msg).await?;
    info!("Successfully submitted create ISM tx with hash {}", res.tx_hash);

    Ok(())
}
