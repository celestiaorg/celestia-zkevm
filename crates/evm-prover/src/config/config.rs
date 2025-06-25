use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const APP_HOME_DIR: &str = ".evm-prover";
pub const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // the prover service grpc server address
    pub grpc_address: String,
    // the genesis path for the EVM chain genesis file
    pub genesis_path: Option<PathBuf>,
}

impl Config {
    pub fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".to_string(),
            genesis_path: Some(PathBuf::from(format!("{}/genesis.json", APP_HOME_DIR))),
        }
    }
}
