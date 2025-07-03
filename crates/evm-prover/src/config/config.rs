use serde::{Deserialize, Serialize};

pub const APP_HOME_DIR: &str = ".evm-prover";
pub const CONFIG_DIR: &str = "config";

pub const CONFIG_FILE: &str = "config.yaml";
pub const GENESIS_FILE: &str = "genesis.json";

pub const DEFAULT_GENESIS_JSON: &str = include_str!("../../resources/genesis.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub grpc_address: String,
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub sequencer_rpc: String,
    pub genesis_path: String,
    pub namespace_hex: String,
}

impl Config {
    pub fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".to_string(),
            celestia_rpc: "127.0.0.1:26658".to_string(),
            evm_rpc: "127.0.0.1:8545".to_string(),
            sequencer_rpc: "http://127.0.0.1:7331".to_string(),
            genesis_path: format!("{}/{}", CONFIG_DIR, GENESIS_FILE),
            namespace_hex: "b7b24d9321578eb83626".to_string(), // default namespace
        }
    }
}
