use serde::{Deserialize, Serialize};

pub const APP_HOME: &str = ".ev-prover";

pub const CONFIG_DIR: &str = "config";

pub const CONFIG_FILE: &str = "config.yaml";
pub const GENESIS_FILE: &str = "genesis.json";

pub const DEFAULT_GENESIS_JSON: &str = include_str!("../../resources/genesis.json");
pub const DEFAULT_NAMESPACE: &str = "a8045f161bf468bf4d44";
pub const DEFAULT_PUB_KEY_HEX: &str = "3964a68700cf76e215626e076e76d23bd1f4c3b31184b5822fd7b4df15d5ce9a";

pub const GROTH16_VK: &[u8] = include_bytes!("../../resources/groth16_vk.bin");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub grpc_address: String,
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub namespace_hex: String,
    pub pub_key: String,
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_queue_capacity() -> usize {
    256
}

fn default_concurrency() -> usize {
    16
}

fn default_batch_size() -> usize {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".to_string(),
            celestia_rpc: "127.0.0.1:26658".to_string(),
            evm_rpc: "http://127.0.0.1:8545".to_string(),
            namespace_hex: DEFAULT_NAMESPACE.to_string(),
            pub_key: DEFAULT_PUB_KEY_HEX.to_string(),
            queue_capacity: default_queue_capacity(),
            concurrency: default_concurrency(),
            batch_size: default_batch_size(),
        }
    }
}
