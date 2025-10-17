use std::fs;

use anyhow::{bail, Result};
use tracing::info;

use crate::commands::cli::{QueryCommands, VERSION};
use crate::config::config::{Config, APP_HOME, CONFIG_DIR, CONFIG_FILE, DEFAULT_GENESIS_JSON, GENESIS_FILE};
use crate::proto::celestia::prover::v1::prover_client::ProverClient;
use crate::proto::celestia::prover::v1::{
    GetBlockProofRequest, GetBlockProofsInRangeRequest, GetLatestBlockProofRequest,
    GetLatestMembershipProofRequest, GetMembershipProofRequest, GetRangeProofsRequest,
};
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

pub async fn query(query_cmd: QueryCommands) -> Result<()> {
    match query_cmd {
        QueryCommands::LatestBlock { server } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!("Querying latest block proof...");
            let response = client.get_latest_block_proof(GetLatestBlockProofRequest {}).await?;
            let inner = response.into_inner();

            if let Some(proof) = inner.proof {
                println!("✓ Found latest block proof:");
                println!("  Height: {}", proof.celestia_height);
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
            } else {
                println!("✗ No proof data returned");
            }
        }
        QueryCommands::Block { height, server } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!("Querying block proof for height {}...", height);
            let response = client
                .get_block_proof(GetBlockProofRequest {
                    celestia_height: height,
                })
                .await?;

            if let Some(proof) = response.into_inner().proof {
                println!("✓ Found block proof:");
                println!("  Height: {}", proof.celestia_height);
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
            } else {
                println!("✗ No proof data returned");
            }
        }
        QueryCommands::BlockRange {
            start_height,
            end_height,
            server,
        } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!(
                "Querying block proofs in range [{}, {}]...",
                start_height, end_height
            );
            let response = client
                .get_block_proofs_in_range(GetBlockProofsInRangeRequest {
                    start_height,
                    end_height,
                })
                .await?;

            let proofs = response.into_inner().proofs;
            println!("✓ Found {} block proof(s):\n", proofs.len());

            for (i, proof) in proofs.iter().enumerate() {
                println!("--- Proof {} ---", i + 1);
                println!("  Height: {}", proof.celestia_height);
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
                println!();
            }
        }
        QueryCommands::LatestMembership { server } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!("Querying latest membership proof...");
            let response = client
                .get_latest_membership_proof(GetLatestMembershipProofRequest {})
                .await?;

            if let Some(proof) = response.into_inner().proof {
                println!("✓ Found latest membership proof:");
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
            } else {
                println!("✗ No proof data returned");
            }
        }
        QueryCommands::Membership { height, server } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!("Querying membership proof for height {}...", height);
            let response = client
                .get_membership_proof(GetMembershipProofRequest { height })
                .await?;

            if let Some(proof) = response.into_inner().proof {
                println!("✓ Found membership proof:");
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
            } else {
                println!("✗ No proof data returned");
            }
        }
        QueryCommands::RangeProofs {
            start_height,
            end_height,
            server,
        } => {
            println!("Connecting to gRPC server at {}...", server);
            let mut client = ProverClient::connect(server).await?;
            println!("✓ Connected\n");

            println!(
                "Querying range proofs for range [{}, {}]...",
                start_height, end_height
            );
            let response = client
                .get_range_proofs(GetRangeProofsRequest {
                    start_height,
                    end_height,
                })
                .await?;

            let proofs = response.into_inner().proofs;
            println!("✓ Found {} range proof(s):\n", proofs.len());

            for (i, proof) in proofs.iter().enumerate() {
                println!("--- Range Proof {} ---", i + 1);
                println!("  Range: {} - {}", proof.start_height, proof.end_height);
                println!("  Proof size: {} bytes", proof.proof_data.len());
                println!("  Public values size: {} bytes", proof.public_values.len());
                println!("  Created at (Unix): {}", proof.created_at);
                println!();
            }
        }
    }

    Ok(())
}
