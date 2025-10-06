//! EVM Client for Hyperlane Warp Route interactions
//!
//! This crate provides functionality to interact with Hyperlane Warp Route contracts
//! on EVM-compatible rollups, particularly for transferring tokens back to Celestia.

use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::sol_types::SolCall;
use anyhow::{Context, Result};
use reqwest::Url;

// Define the Hyperlane Warp Route contract interface
sol! {
    #[sol(rpc)]
    contract HyperlaneWarpRoute {
        function transferRemote(
            uint32 destination,
            bytes32 recipient,
            uint256 amount
        ) external returns (bytes32 messageId);
    }
}

/// Configuration for the EVM client
#[derive(Debug, Clone)]
pub struct EvmClientConfig {
    /// RPC URL for the EVM rollup
    pub rpc_url: String,
    /// Private key for signing transactions (hex encoded)
    pub private_key: String,
    /// Warp Route contract address
    pub contract_address: Address,
}

impl EvmClientConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            rpc_url: std::env::var("EVM_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string()),
            private_key: std::env::var("EVM_PRIVATE_KEY").context("EVM_PRIVATE_KEY environment variable not set")?,
            contract_address: std::env::var("WARP_CONTRACT_ADDRESS")
                .unwrap_or_else(|_| "0x345a583028762De4d733852c9D4f419077093A48".to_string())
                .parse()
                .context("Invalid WARP_CONTRACT_ADDRESS")?,
        })
    }
}

/// EVM client for interacting with Hyperlane Warp Routes
pub struct EvmClient {
    config: EvmClientConfig,
}

impl EvmClient {
    /// Create a new EVM client
    pub fn new(config: EvmClientConfig) -> Self {
        Self { config }
    }

    /// Create a client from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self::new(EvmClientConfig::from_env()?))
    }

    /// Transfer tokens back to Celestia via Hyperlane
    ///
    /// # Arguments
    /// * `destination_domain` - Celestia domain ID (e.g., 69420)
    /// * `recipient` - Recipient address on Celestia (32 bytes)
    /// * `amount` - Amount to transfer
    ///
    /// # Returns
    /// Transaction hash as hex string
    pub async fn transfer_remote(
        &self,
        destination_domain: u32,
        recipient: FixedBytes<32>,
        amount: U256,
    ) -> Result<String> {
        // Parse private key
        let signer: PrivateKeySigner = self
            .config
            .private_key
            .trim_start_matches("0x")
            .parse()
            .context("Failed to parse private key")?;

        // Create wallet
        let wallet = EthereumWallet::from(signer);

        // Create provider with HTTP connection and wallet
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect(&self.config.rpc_url)
            .await?;

        // Encode the function call
        let call_data = HyperlaneWarpRoute::transferRemoteCall {
            destination: destination_domain,
            recipient,
            amount,
        }
        .abi_encode();

        // Build transaction
        let tx = TransactionRequest::default()
            .to(self.config.contract_address)
            .with_input(call_data);

        // Send transaction
        let pending_tx = provider
            .send_transaction(tx)
            .await
            .context("Failed to send transaction")?;

        let tx_hash = *pending_tx.tx_hash();

        // Wait for confirmation
        let receipt = pending_tx
            .get_receipt()
            .await
            .context("Failed to get transaction receipt")?;

        if !receipt.status() {
            anyhow::bail!("Transaction failed with status: {:?}", receipt.status());
        }

        Ok(format!("{:?}", tx_hash))
    }

    /// Get the contract address
    pub fn contract_address(&self) -> Address {
        self.config.contract_address
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.config.rpc_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config = EvmClientConfig {
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a".to_string(),
            contract_address: "0x345a583028762De4d733852c9D4f419077093A48".parse().unwrap(),
        };

        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(
            format!("{:?}", config.contract_address),
            "0x345a583028762De4d733852c9D4f419077093A48"
        );
    }
}
