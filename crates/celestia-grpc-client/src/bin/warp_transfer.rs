#!/usr/bin/env cargo

use anyhow::Result;
use celestia_grpc::{GrpcClient, TxConfig};
use celestia_grpc_client::MsgWarpTransfer;
use clap::Parser;
use tracing::{info, Level};

#[derive(Parser)]
#[command(author, version, about = "Send Hyperlane Warp token transfers", long_about = None)]
struct Cli {
    /// Token ID (32 bytes hex encoded)
    #[arg(long)]
    token_id: String,

    /// Destination domain ID
    #[arg(long)]
    destination_domain: u32,

    /// Recipient address (hex encoded)
    #[arg(long)]
    recipient: String,

    /// Amount to transfer
    #[arg(long)]
    amount: String,

    /// Gas limit (default: 200000)
    #[arg(long, default_value = "200000")]
    gas_limit: u64,

    /// Gas price in utia (default: 800)
    #[arg(long, default_value = "800")]
    gas_price: u64,

    /// Maximum Hyperlane fee in utia (default: 100)
    #[arg(long, default_value = "100")]
    max_hyperlane_fee: u64,

    /// Transaction fees (e.g., "800utia")
    #[arg(long)]
    fees: Option<String>,
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

    // Get configuration from environment
    let grpc_endpoint = std::env::var("CELESTIA_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:9090".to_string());
    let private_key_hex = std::env::var("CELESTIA_PRIVATE_KEY")
        .map_err(|_| anyhow::anyhow!("CELESTIA_PRIVATE_KEY environment variable not set"))?;

    info!("Connecting to Celestia gRPC endpoint: {}", grpc_endpoint);

    // Create gRPC client
    let tx_client = GrpcClient::builder()
        .url(&grpc_endpoint)
        .private_key_hex(&private_key_hex)
        .build()?;

    // Derive signer address
    let signer_address = derive_signer_address(&private_key_hex)?;
    info!("Using signer address: {}", signer_address);

    // Create warp transfer message
    let msg = MsgWarpTransfer::new(
        signer_address,
        cli.token_id.clone(),
        cli.destination_domain,
        cli.recipient.clone(),
        cli.amount.clone(),
        cli.max_hyperlane_fee.to_string(),
    );

    info!(
        "Submitting warp transfer: token_id={}, destination_domain={}, recipient={}, amount={}",
        cli.token_id, cli.destination_domain, cli.recipient, cli.amount
    );

    // Configure transaction
    let tx_config = TxConfig {
        gas_limit: Some(cli.gas_limit),
        gas_price: Some(cli.gas_price as f64),
        memo: Some(format!(
            "Warp transfer to domain {} (max_hyperlane_fee: {}utia)",
            cli.destination_domain, cli.max_hyperlane_fee
        )),
        ..Default::default()
    };

    // Submit the transaction
    let tx_info = tx_client.submit_message(msg, tx_config).await?;

    println!("✓ Warp transfer submitted successfully!");
    println!("  Transaction hash: {}", tx_info.hash);
    println!("  Block height: {}", tx_info.height.value());
    println!("  Token ID: {}", cli.token_id);
    println!("  Destination domain: {}", cli.destination_domain);
    println!("  Recipient: {}", cli.recipient);
    println!("  Amount: {}", cli.amount);

    Ok(())
}

/// Derive bech32 signer address from private key
fn derive_signer_address(private_key_hex: &str) -> Result<String> {
    use bech32::{Bech32, Hrp};
    use ed25519_dalek::SigningKey;
    use sha2::{Digest, Sha256};

    // Remove 0x prefix if present
    let key_hex = private_key_hex.trim_start_matches("0x");

    // Decode private key
    let key_bytes = hex::decode(key_hex)?;
    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!("Private key must be 32 bytes"));
    }

    // Create signing key and get public key
    let signing_key = SigningKey::from_bytes(
        &key_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid private key length"))?,
    );
    let public_key = signing_key.verifying_key();

    // Hash public key with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let hash = hasher.finalize();

    // Take first 20 bytes for address
    let address_bytes = &hash[..20];

    // Encode as bech32 with "celestia" prefix
    let hrp = Hrp::parse("celestia")?;
    let address = bech32::encode::<Bech32>(hrp, address_bytes)?;

    Ok(address)
}
