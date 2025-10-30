use std::fs;
use std::path::PathBuf;

use alloy_genesis::Genesis as AlloyGenesis;
use anyhow::{anyhow, Context, Result};
use celestia_types::nmt::Namespace;
use rsp_primitives::genesis::Genesis;
use serde::{Deserialize, Serialize};
use tracing::info;

pub const DEFAULT_NAMESPACE: &str = "a8045f161bf468bf4d44";
pub const DEFAULT_PUB_KEY_HEX: &str = "3964a68700cf76e215626e076e76d23bd1f4c3b31184b5822fd7b4df15d5ce9a";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The local gRPC listen address for this service.
    pub grpc_address: String,

    /// Configuration for outbound RPC connections (Celestia, EVM, etc.)
    pub rpc: RpcConfig,

    /// Namespace ID for Celestia blob inclusion.
    pub namespace: Namespace,

    /// Sequencer’s public key in hex.
    pub pub_key: String,

    /// Capacity for internal proof event queue.
    pub queue_capacity: usize,

    /// Maximum concurrent proof tasks.
    pub concurrency: usize,

    /// Number of blocks per range proof.
    pub batch_size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RpcConfig {
    /// RPC endpoint for the Celestia light node.
    pub celestia_rpc: String,

    /// RPC endpoint for the sequencer node.
    pub evnode_rpc: String,

    /// RPC endpoint for the EVM node.
    pub evreth_rpc: String,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            celestia_rpc: "http://localhost:26658".into(),
            evnode_rpc: "http://localhost:7331".into(),
            evreth_rpc: "http://localhost:8545".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".into(),
            rpc: RpcConfig::default(),
            namespace: Namespace::new_v0(&hex::decode(DEFAULT_NAMESPACE).unwrap()).unwrap(),
            pub_key: DEFAULT_PUB_KEY_HEX.into(),
            queue_capacity: 256,
            concurrency: 16,
            batch_size: 10,
        }
    }
}

impl Config {
    /// The default service home directory.
    pub const APP_HOME: &str = ".ev-prover";
    /// The default configuration directory.
    pub const CONFIG_DIR: &str = "config";
    /// The default configuration file in YAML format.
    pub const CONFIG_FILE: &str = "config.yaml";
    /// The chain genesis file.
    pub const GENESIS_FILE: &str = "genesis.json";
    /// Default chain genesis file used for testing purposes.
    const DEFAULT_GENESIS_JSON: &str = include_str!("../../resources/genesis.json");
    /// The groth16 verifier key.
    const GROTH16_VK: &[u8] = include_bytes!("../../resources/groth16_vk.bin");

    /// Initializes the local configuration directory and writes default files if missing.
    pub fn init() -> Result<()> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("cannot find home directory"))?
            .join(Self::APP_HOME);

        fs::create_dir_all(&home_dir)?;
        let config_dir = home_dir.join(Self::CONFIG_DIR);
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join(Self::CONFIG_FILE);
        if !config_path.exists() {
            info!("creating default config at {config_path:?}");
            let yaml = serde_yaml::to_string(&Config::default())?;
            fs::write(&config_path, yaml)?;
        } else {
            info!("config file already exists at {config_path:?}");
        }

        let genesis_path = config_dir.join(Self::GENESIS_FILE);
        if !genesis_path.exists() {
            info!("writing embedded genesis to {genesis_path:?}");
            fs::write(&genesis_path, Self::DEFAULT_GENESIS_JSON)?;
        }

        Ok(())
    }

    /// Returns the default application config path.
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .expect("cannot find home directory")
            .join(Self::APP_HOME)
            .join(Self::CONFIG_DIR)
            .join(Self::CONFIG_FILE)
    }

    /// Returns the default chain genesis path.
    pub fn genesis_path() -> PathBuf {
        dirs::home_dir()
            .expect("cannot find home directory")
            .join(Self::APP_HOME)
            .join(Self::CONFIG_DIR)
            .join(Self::GENESIS_FILE)
    }

    /// Returns the groth16 verifiying key.
    pub fn groth16_vkey() -> Vec<u8> {
        Self::GROTH16_VK.to_vec()
    }

    /// Loads the application config from the service home directory.
    pub fn load() -> Result<Self> {
        let config_path = Config::config_path();
        if !config_path.exists() {
            return Err(anyhow!("config file not found at {}", config_path.display()));
        }

        info!("reading config file at {}", config_path.display());
        let config_yaml = fs::read_to_string(&config_path).context("Failed to read config file from path")?;
        let config = serde_yaml::from_str(&config_yaml)?;

        Ok(config)
    }

    /// Loads the chain genesis from the service home directory.
    pub fn load_genesis() -> Result<Genesis> {
        let genesis_path = Config::genesis_path();
        if !genesis_path.exists() {
            return Err(anyhow!("genesis file not found at {}", genesis_path.display()));
        }

        info!("reading genesis file at {}", genesis_path.display());
        let genesis_json = fs::read_to_string(genesis_path).context("Failed to read genesis file from path")?;
        let alloy_genesis: AlloyGenesis = serde_json::from_str(&genesis_json)?;

        let genesis = Genesis::Custom(alloy_genesis.config);
        Ok(genesis)
    }
}
