#!/usr/bin/env cargo

use anyhow::Result;
use celestia_grpc_client::proto::celestia::zkism::v1::{QueryIsmRequest, QueryIsmsRequest};
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{CelestiaIsmClient, MsgRemoteTransfer, StateInclusionProofMsg, StateTransitionProofMsg};
use clap::{Parser, Subcommand};
use tracing::{info, Level};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit a state transition proof (MsgUpdateZKExecutionISM)
    StateTransition {
        /// ISM identifier
        #[arg(long)]
        id: String,
        /// Proof file path (hex encoded)
        #[arg(long)]
        proof_file: String,
        /// Public values file path (hex encoded)
        #[arg(long)]
        public_values_file: String,
    },
    /// Submit a state inclusion proof (MsgSubmitMessages)
    StateInclusion {
        /// ISM identifier
        #[arg(long)]
        id: String,
        /// Proof file path (hex encoded)
        #[arg(long)]
        proof_file: String,
        /// Public values file path (hex encoded)
        #[arg(long)]
        public_values_file: String,
        /// Block height for inclusion proof
        #[arg(long)]
        height: u64,
    },
    Transfer {
        /// The sender address (must be the tx signer)
        #[arg(long)]
        sender: String,
        /// The Hyperlane warp token identifier
        #[arg(long)]
        token_id: String,
        /// The destination domain for the transfer (e.g. 1234)
        #[arg(long)]
        domain: u32,
        /// The recipient address on the counterparty
        #[arg(long)]
        recipient: String,
        // The token amount
        #[arg(long)]
        amount: String,
    },
    QueryISM {
        /// ISM identifier
        #[arg(long)]
        id: String,
    },
    QueryISMS {},
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install default crypto provider"))?;

    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();

    let config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(config).await?;

    info!("CelestiaISMClient using signer address: {}", client.signer_address());

    match &cli.command {
        Commands::StateTransition {
            id,
            proof_file,
            public_values_file,
        } => {
            info!("Submitting state transition proof (MsgUpdateZKExecutionISM)...");

            let proof = read_hex_file(proof_file)?;
            let public_values = read_hex_file(public_values_file)?;
            let signer_address = client.signer_address().to_string();

            let proof_msg = StateTransitionProofMsg::new(id.clone(), proof, public_values, signer_address);

            let response = client.send_tx(proof_msg).await?;
            println!("State transition proof submitted successfully!");
            println!("Transaction hash: {}", response.tx_hash);
            println!("Block height: {}", response.height);
            println!("Gas used: {}", response.gas_used);
        }
        Commands::StateInclusion {
            id,
            proof_file,
            public_values_file,
            height,
        } => {
            info!("Submitting state inclusion proof (MsgSubmitMessages)...");

            let proof = read_hex_file(proof_file)?;
            let public_values = read_hex_file(public_values_file)?;
            let signer_address = client.signer_address().to_string();

            let proof_msg = StateInclusionProofMsg::new(id.clone(), *height, proof, public_values, signer_address);

            let response = client.send_tx(proof_msg).await?;
            println!("State inclusion proof submitted successfully!");
            println!("Transaction hash: {}", response.tx_hash);
            println!("Block height: {}", response.height);
            println!("Gas used: {}", response.gas_used);
        }
        Commands::Transfer {
            sender,
            token_id,
            domain,
            recipient,
            amount,
        } => {
            info!("Submitting warp transfer (MsgRemoteTransfer)...");

            let transfer_msg = MsgRemoteTransfer::new(
                sender.clone(),
                token_id.clone(),
                *domain,
                recipient.clone(),
                amount.clone(),
            );

            let response = client.send_tx(transfer_msg).await?;
            println!("Warp transfer submitted successfully!");
            println!("Transaction hash: {}", response.tx_hash);
            println!("Block height: {}", response.height);
            println!("Gas used: {}", response.gas_used);
        }
        Commands::QueryISM { id } => {
            info!("Querying zk ism with id: {id}");

            let query_msg = QueryIsmRequest { id: id.clone() };
            let response = client.ism(query_msg).await?;
            println!("Response = {response:?}");
        }
        Commands::QueryISMS {} => {
            info!("Querying zk isms");

            let query_msg = QueryIsmsRequest { pagination: None };
            let response = client.isms(query_msg).await?;
            println!("Response = {response:?}");
        }
    }

    Ok(())
}

fn read_hex_file(file_path: &str) -> Result<Vec<u8>> {
    let content = std::fs::read_to_string(file_path)?;
    let hex_content = content.trim();
    let bytes = hex::decode(hex_content)?;
    Ok(bytes)
}
