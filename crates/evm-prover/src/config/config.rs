use serde::{Deserialize, Serialize};

pub const APP_HOME_DIR: &str = ".evm-prover";
pub const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub grpc_address: String,
}

impl Config {
    pub fn default() -> Self {
        Self {
            grpc_address: "127.0.0.1:50051".to_string(),
        }
    }
}
