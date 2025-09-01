use serde::{Deserialize, Serialize};

pub const APP_HOME: &str = ".evm-prover";
pub const CONFIG_DIR: &str = "config";

pub const CONFIG_FILE: &str = "config.yaml";
pub const GENESIS_FILE: &str = "genesis.json";

pub const DEFAULT_GENESIS_JSON: &str = include_str!("../../resources/genesis.json");
pub const DEFAULT_NAMESPACE: &str = "a8045f161bf468bf4d44";
pub const DEFAULT_PUB_KEY_HEX: &str = "3964a68700cf76e215626e076e76d23bd1f4c3b31184b5822fd7b4df15d5ce9a";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub grpc_address: String,
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub namespace_hex: String,
    pub pub_key: String,
}

impl Config {
    pub fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".to_string(),
            celestia_rpc: "127.0.0.1:26658".to_string(),
            evm_rpc: "http://127.0.0.1:8545".to_string(),
            namespace_hex: DEFAULT_NAMESPACE.to_string(),
            pub_key: DEFAULT_PUB_KEY_HEX.to_string(),
        }
    }
}
