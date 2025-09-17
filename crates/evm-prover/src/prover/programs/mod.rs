use alloy_primitives::{hex::FromHex, FixedBytes};
use alloy_provider::Provider;
use anyhow::Result;
use async_trait::async_trait;
use evm_storage_proofs::client::EvmClient;

use crate::prover::programs::types::DefaultProvider;

pub mod block;
pub mod message;
pub mod range;
pub mod types;

#[async_trait]
pub trait StateQueryProvider: Send + Sync {
    async fn get_state_root(&self, height: u64) -> Result<FixedBytes<32>>;
    async fn get_height(&self) -> u64;
}

pub struct MockStateQueryProvider {
    provider: DefaultProvider,
    client: EvmClient,
}
impl MockStateQueryProvider {
    pub fn new(provider: DefaultProvider, client: EvmClient) -> Self {
        Self { provider, client }
    }
}

#[async_trait]
impl StateQueryProvider for MockStateQueryProvider {
    async fn get_state_root(&self, height: u64) -> Result<FixedBytes<32>> {
        Ok(FixedBytes::from_hex(&self.client.get_state_root(height).await?).expect("Failed to get state root"))
    }
    async fn get_height(&self) -> u64 {
        self.provider.get_block_number().await.expect("Failed to get height")
    }
}
