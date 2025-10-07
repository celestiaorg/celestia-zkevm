use serde::{Deserialize, Serialize};

/// Response from proof submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxResponse {
    /// Transaction hash
    pub tx_hash: String,
    /// Block height where transaction was included
    pub height: u64,
    /// Gas used for the transaction
    pub gas_used: u64,
    /// Whether the transaction was successful
    pub success: bool,
    /// Error message if transaction failed
    pub error_message: Option<String>,
}

/// Configuration for the Celestia proof client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Celestia validator gRPC endpoint
    pub grpc_endpoint: String,
    /// Private key for signing transactions (hex encoded)
    pub private_key_hex: String,
    /// Cached bech32-encoded signer address (derived from private key)
    pub signer_address: String,
    /// Chain ID for the Celestia network
    pub chain_id: String,
    /// Gas price for transactions
    pub gas_price: u64,
    /// Maximum gas limit per transaction
    pub max_gas: u64,
    /// Timeout for transaction confirmation (in seconds)
    pub confirmation_timeout: u64,
}

impl ClientConfig {
    /// Derive the bech32-encoded signer address from the private key
    pub fn derive_signer_address(private_key_hex: &str) -> Result<String, anyhow::Error> {
        use anyhow::Context;
        use bech32::{self, Bech32, Hrp};
        use k256::ecdsa::SigningKey;
        use ripemd::Ripemd160;
        use sha2::{Digest, Sha256};

        let sk_bytes = hex::decode(private_key_hex).context("Failed to decode private key hex")?;
        let signing_key = SigningKey::from_slice(&sk_bytes).context("Failed to create signing key from bytes")?;

        let vk = signing_key.verifying_key();
        let pk_compressed = vk.to_encoded_point(true);

        let sha = Sha256::digest(pk_compressed.as_bytes());
        let ripemd = Ripemd160::digest(&sha);
        let hrp = Hrp::parse("celestia")?;
        let addr = bech32::encode::<Bech32>(hrp, ripemd.as_slice()).context("Failed to encode bech32 address")?;

        Ok(addr)
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            grpc_endpoint: "http://localhost:9090".to_string(),
            private_key_hex: String::new(),
            signer_address: String::new(),
            chain_id: "celestia-zkevm-testnet".to_string(),
            gas_price: 1000,
            max_gas: 200_000,
            confirmation_timeout: 60,
        }
    }
}
