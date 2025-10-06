#!/usr/bin/env cargo

use alloy::primitives::{FixedBytes, U256};
use anyhow::Result;
use clap::Parser;
use ev_client::EvmClient;
use tracing::{info, Level};

#[derive(Parser)]
#[command(author, version, about = "Transfer tokens from EVM rollup back to Celestia", long_about = None)]
struct Cli {
    /// Destination domain ID (Celestia domain, e.g., 69420)
    #[arg(long, default_value = "69420")]
    destination_domain: u32,

    /// Recipient address on Celestia (hex encoded, 32 bytes)
    #[arg(long)]
    recipient: String,

    /// Amount to transfer
    #[arg(long)]
    amount: u64,

    /// Contract address (optional, can be set via WARP_CONTRACT_ADDRESS env var)
    #[arg(long)]
    contract_address: Option<String>,

    /// RPC URL (optional, can be set via EVM_RPC_URL env var)
    #[arg(long)]
    rpc_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();

    // Override environment variables if CLI args provided
    if let Some(ref rpc_url) = cli.rpc_url {
        std::env::set_var("EVM_RPC_URL", rpc_url);
    }
    if let Some(ref contract_address) = cli.contract_address {
        std::env::set_var("WARP_CONTRACT_ADDRESS", contract_address);
    }

    let client = EvmClient::from_env()?;

    info!("Using EVM RPC: {}", client.rpc_url());
    info!("Warp contract: {:?}", client.contract_address());

    let recipient_hex = cli.recipient.trim_start_matches("0x");
    let recipient_bytes = hex::decode(recipient_hex)?;

    if recipient_bytes.len() != 32 {
        anyhow::bail!(
            "Recipient must be 32 bytes, got {} bytes. Pad with leading zeros if needed.",
            recipient_bytes.len()
        );
    }

    let recipient: FixedBytes<32> = FixedBytes::from_slice(&recipient_bytes);
    let amount = U256::from(cli.amount);

    info!(
        "Transferring {} tokens to domain {} (recipient: 0x{})",
        cli.amount,
        cli.destination_domain,
        hex::encode(recipient.as_slice())
    );

    let tx_hash = client
        .transfer_remote(cli.destination_domain, recipient, amount)
        .await?;

    println!("✓ Transfer transaction submitted successfully!");
    println!("  Transaction hash: {}", tx_hash);
    println!("  Destination domain: {}", cli.destination_domain);
    println!("  Recipient: 0x{}", hex::encode(recipient.as_slice()));
    println!("  Amount: {}", cli.amount);

    Ok(())
}
