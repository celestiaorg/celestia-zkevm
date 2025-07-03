use std::fs;

use anyhow::{bail, Result};

use crate::commands::cli::VERSION;
use crate::config::config::{Config, APP_HOME_DIR, CONFIG_DIR, CONFIG_FILE, DEFAULT_GENESIS_JSON, GENESIS_FILE};
use crate::grpc::server::create_grpc_server;

pub fn init() -> Result<()> {
    let home_dir = dirs::home_dir().expect("cannot find home directory").join(APP_HOME_DIR);

    if !home_dir.exists() {
        println!("creating home directory at {:?}", home_dir);
        fs::create_dir_all(&home_dir)?;
    }

    let config_dir = home_dir.join(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        println!("creating default config at {:?}", config_path);
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config)?;
        fs::write(config_path, yaml)?;
    } else {
        println!("config file already exists at {:?}", config_path);
    }

    let genesis_path = config_dir.join(GENESIS_FILE);
    if !genesis_path.exists() {
        println!("writing embedded genesis to {:?}", genesis_path);
        fs::write(&genesis_path, DEFAULT_GENESIS_JSON)?;
    }

    Ok(())
}

pub async fn start() -> Result<()> {
    let config_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME_DIR)
        .join(CONFIG_DIR)
        .join(CONFIG_FILE);

    if !config_path.exists() {
        bail!("config file not found at {}", config_path.display());
    }

    let config_yaml = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&config_yaml)?;

    println!("starting gRPC server at {}", config.grpc_address);
    create_grpc_server(config).await?;

    Ok(())
}

pub fn version() {
    println!("version: {}", VERSION);
}
