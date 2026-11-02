use std::path::PathBuf;
use std::{env, fs};

use alloy_genesis::Genesis as AlloyGenesis;
use anyhow::{anyhow, Context, Result};
use celestia_types::nmt::Namespace;
use rsp_primitives::genesis::Genesis;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

pub const DEFAULT_NAMESPACE: &str = "a8045f161bf468bf4d44";
pub const DEFAULT_PUB_KEY_HEX: &str = "3964a68700cf76e215626e076e76d23bd1f4c3b31184b5822fd7b4df15d5ce9a";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The local gRPC listen address for this service.
    pub grpc_address: String,

    /// Configuration for outbound RPC connections (Celestia, EVM, etc.)
    pub rpc: RpcConfig,

    /// Configuration for Hyperlane addresses and identifiers.
    pub hyperlane: HyperlaneConfig,

    /// Namespace ID for Celestia blob inclusion.
    pub namespace: Namespace,

    /// Sequencer’s public key in hex.
    pub pub_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".into(),
            rpc: RpcConfig::default(),
            hyperlane: HyperlaneConfig::default(),
            namespace: Namespace::new_v0(&hex::decode(DEFAULT_NAMESPACE).unwrap()).unwrap(),
            pub_key: DEFAULT_PUB_KEY_HEX.into(),
        }
    }
}

impl Config {
    /// Builds a config from the environment, falling back to the defaults for anything unset.
    ///
    /// The Hyperlane EVM addresses have no meaningful default: the docker testnet generates
    /// them on every run (testnet/hyperlane/generated/deployed.env), so they must come from
    /// MAILBOX_CONTRACT_ADDRESS and MERKLE_TREE_CONTRACT_ADDRESS.
    pub fn from_env() -> Result<Self> {
        let evm = match EvmHyperlaneConfig::from_env() {
            Ok(evm) => evm,
            Err(err) => {
                warn!("{err:#}; writing placeholder EVM addresses. Source testnet/hyperlane/generated/deployed.env and re-run init.");
                EvmHyperlaneConfig::default()
            }
        };
        Ok(Self {
            rpc: RpcConfig::from_env()?,
            hyperlane: HyperlaneConfig {
                evm,
                celestia: CelestiaHyperlaneConfig::from_env()?,
            },
            ..Self::default()
        })
    }

    /// The default service home directory.
    pub const APP_HOME: &str = ".ev-prover";
    /// The default configuration directory.
    pub const CONFIG_DIR: &str = "config";
    /// The default configuration file in YAML format.
    pub const CONFIG_FILE: &str = "config.yaml";
    /// The default data directory.
    pub const DATA_DIR: &str = "data";
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
            info!("Creating config from the environment at {config_path:?}");
            let yaml = serde_yaml::to_string(&Config::from_env()?)?;
            fs::write(&config_path, yaml)?;
        } else {
            info!("Config file already exists at {config_path:?}; delete it to regenerate from the environment");
        }

        let genesis_path = config_dir.join(Self::GENESIS_FILE);
        if !genesis_path.exists() {
            info!("Writing embedded genesis to {genesis_path:?}");
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

    pub fn storage_path() -> PathBuf {
        dirs::home_dir()
            .expect("cannot find home directory")
            .join(Self::APP_HOME)
            .join(Self::DATA_DIR)
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

        info!("Reading config file at {}", config_path.display());
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

        info!("Reading genesis file at {}", genesis_path.display());
        let genesis_json = fs::read_to_string(genesis_path).context("Failed to read genesis file from path")?;
        let alloy_genesis: AlloyGenesis = serde_json::from_str(&genesis_json)?;

        let genesis = Genesis::Custom(alloy_genesis.config);
        Ok(genesis)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RpcConfig {
    /// RPC endpoint for the data availability node.
    pub celestia_rpc: String,

    /// RPC endpoint for the consensus node.
    pub tendermint_rpc: String,

    /// Authentication token for the Celestia RPC client.
    /// If None, no authentication is used.
    pub celestia_auth_token: Option<String>,

    /// RPC endpoint for the sequencer node.
    pub evnode_rpc: String,

    /// RPC endpoint for the EVM node.
    pub evreth_rpc: String,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            celestia_rpc: "http://localhost:26658".into(),
            tendermint_rpc: "http://localhost:26657".into(),
            celestia_auth_token: None,
            evnode_rpc: "http://localhost:7331".into(),
            evreth_rpc: "http://localhost:8545".into(),
        }
    }
}

impl RpcConfig {
    pub fn from_env() -> Result<Self> {
        let celestia_rpc = env::var("CELESTIA_RPC_URL").unwrap_or_else(|_| "http://localhost:26658".to_string());
        let tendermint_rpc = env::var("TENDERMINT_RPC_URL").unwrap_or_else(|_| "http://localhost:26657".to_string());
        let celestia_auth_token = env::var("CELESTIA_AUTH_TOKEN").ok();
        let evnode_rpc = env::var("SEQUENCER_RPC_URL").unwrap_or_else(|_| "http://localhost:7331".to_string());
        let evreth_rpc = env::var("RETH_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());

        Ok(Self {
            celestia_rpc,
            tendermint_rpc,
            celestia_auth_token,
            evnode_rpc,
            evreth_rpc,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct HyperlaneConfig {
    #[serde(default)]
    pub evm: EvmHyperlaneConfig,

    #[serde(default)]
    pub celestia: CelestiaHyperlaneConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EvmHyperlaneConfig {
    /// Mailbox contract address on the EVM chain.
    pub mailbox_address: String,

    /// Merkle tree contract address on the EVM chain.
    pub merkle_tree_address: String,
}

impl EvmHyperlaneConfig {
    pub fn from_env() -> Result<Self> {
        let mailbox_address = env::var("MAILBOX_CONTRACT_ADDRESS").context("MAILBOX_CONTRACT_ADDRESS must be set")?;
        let merkle_tree_address =
            env::var("MERKLE_TREE_CONTRACT_ADDRESS").context("MERKLE_TREE_CONTRACT_ADDRESS must be set")?;
        Ok(Self {
            mailbox_address,
            merkle_tree_address,
        })
    }
}

impl Default for EvmHyperlaneConfig {
    /// Placeholders: the testnet generates these addresses on every run, see [`Config::from_env`].
    fn default() -> Self {
        Self {
            mailbox_address: "0xa05915fd6e32a1aa7e67d800164cacb12487142d".into(),
            merkle_tree_address: "0x6007cE81D2FD7b9b7f22e71cE9896e00d6017ba8".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CelestiaHyperlaneConfig {
    /// Ism identifier
    pub ism_id: String,

    /// Celestia mailbox identifier / address (whatever format you use).
    pub mailbox_id: String,
}

impl CelestiaHyperlaneConfig {
    pub fn from_env() -> Result<Self> {
        let ism_id = env::var("CELESTIA_ISM_ID")
            .unwrap_or_else(|_| "0x726f757465725f69736d000000000000000000000000002a0000000000000002".to_string());
        let mailbox_id = env::var("CELESTIA_MAILBOX_ADDRESS")
            .unwrap_or_else(|_| "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string());
        Ok(Self { ism_id, mailbox_id })
    }
}

impl Default for CelestiaHyperlaneConfig {
    fn default() -> Self {
        Self {
            ism_id: "0x726f757465725f69736d000000000000000000000000002a0000000000000002".to_string(),
            mailbox_id: "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string(),
        }
    }
}
