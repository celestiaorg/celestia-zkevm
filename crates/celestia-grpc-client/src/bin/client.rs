#!/usr/bin/env cargo

use anyhow::Result;
use celestia_grpc_client::proto::celestia::zkism::v1::{query_client::QueryClient, QueryIsmRequest, QueryIsmsRequest};
use celestia_grpc_client::{CelestiaIsmClient, ProofSubmitter, StateInclusionProofMsg, StateTransitionProofMsg};
use clap::{Parser, Subcommand};
use tonic::Request;
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
        /// Block height for state transition
        #[arg(long)]
        height: u64,
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

    // Create client from environment variables
    let client = CelestiaIsmClient::from_env().await?;

    match &cli.command {
        Commands::StateTransition {
            id,
            proof_file,
            public_values_file,
            height,
        } => {
            info!("Submitting state transition proof (MsgUpdateZKExecutionISM)...");

            let proof = read_hex_file(proof_file)?;
            let public_values = read_hex_file(public_values_file)?;
            let signer_address = client.signer_address().to_string();

            let proof_msg = StateTransitionProofMsg::new(id.clone(), *height, proof, public_values, signer_address);

            let response = client.submit_state_transition_proof(proof_msg).await?;
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

            let response = client.submit_state_inclusion_proof(proof_msg).await?;
            println!("State inclusion proof submitted successfully!");
            println!("Transaction hash: {}", response.tx_hash);
            println!("Block height: {}", response.height);
            println!("Gas used: {}", response.gas_used);
        }
        Commands::QueryISM { id } => {
            info!("Querying zk ism with id: {id}");

            let mut query_client = QueryClient::connect("http://127.0.0.1:9090").await?;

            let query_msg = QueryIsmRequest { id: id.clone() };

            let request = tonic::Request::new(query_msg);
            let response = query_client.ism(request).await?;

            println!("Response = {:?}", response.into_inner());
        }
        Commands::QueryISMS {} => {
            info!("Querying zk isms");

            let mut query_client = QueryClient::connect("http://127.0.0.1:9090").await?;

            let query_msg = QueryIsmsRequest { pagination: None };

            let request = Request::new(query_msg);
            let response = query_client.isms(request).await?;

            println!("Response = {:?}", response.into_inner());
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
