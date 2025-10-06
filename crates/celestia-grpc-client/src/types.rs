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
        use bech32;
        use k256::elliptic_curve::consts::U32;
        use k256::elliptic_curve::generic_array::GenericArray;
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        use k256::SecretKey;
        use sha2::{Digest, Sha256};

        // Decode private key
        let private_key_bytes = hex::decode(private_key_hex).context("Failed to decode private key hex")?;

        // Create secret key
        let private_key_array: GenericArray<u8, U32> = GenericArray::clone_from_slice(&private_key_bytes);
        let secret_key = SecretKey::from_bytes(&private_key_array).context("Failed to create secret key from bytes")?;

        // Derive public key
        let public_key = secret_key.public_key();
        let public_key_bytes = public_key.to_encoded_point(false);

        // Hash public key (Cosmos SDK standard)
        let mut hasher = Sha256::new();
        hasher.update(&public_key_bytes.as_bytes()[1..]); // Skip the 0x04 prefix
        let hash = hasher.finalize();

        // Take first 20 bytes for the address
        let address_bytes = &hash[..20];

        // Encode as bech32 with "celestia" prefix
        let bech32_address = bech32::encode::<bech32::Bech32m>(bech32::Hrp::parse("celestia")?, address_bytes)
            .context("Failed to create bech32 address")?;

        Ok(bech32_address)
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
