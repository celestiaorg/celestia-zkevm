use std::fs;

use crate::commands::cli::VERSION;
use crate::config::config::{APP_HOME_DIR, CONFIG_FILE, Config};
use crate::grpc::server::create_grpc_server;

pub fn init() -> anyhow::Result<()> {
    let home_dir = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME_DIR);

    if !home_dir.exists() {
        println!("creating home directory at {:?}", home_dir);
        fs::create_dir_all(&home_dir)?;
    }

    let config_path = home_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        println!("creating default config at {:?}", config_path);
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config)?;
        fs::write(config_path, yaml)?;
    } else {
        println!("config file already exists at {:?}", config_path);
    }

    Ok(())
}

pub async fn start() -> anyhow::Result<()> {
    let config_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME_DIR)
        .join(CONFIG_FILE);

    let config_yaml = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&config_yaml)?;

    println!("starting gRPC server at {}", config.grpc_address);
    create_grpc_server(config).await?;

    Ok(())
}

pub fn version() {
    println!("version: {}", VERSION);
}
