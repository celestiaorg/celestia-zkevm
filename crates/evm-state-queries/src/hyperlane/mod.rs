use alloy_primitives::{FixedBytes, hex::FromHex};
use anyhow::Result;
use async_trait::async_trait;
use evm_storage_proofs::client::EvmClient;
pub mod indexer;
use alloy_provider::{Provider, fillers::FillProvider};
pub type DefaultProvider = FillProvider<
    alloy_provider::fillers::JoinFill<
        alloy_provider::Identity,
        alloy_provider::fillers::JoinFill<
            alloy_provider::fillers::GasFiller,
            alloy_provider::fillers::JoinFill<
                alloy_provider::fillers::BlobGasFiller,
                alloy_provider::fillers::JoinFill<
                    alloy_provider::fillers::NonceFiller,
                    alloy_provider::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy_provider::RootProvider,
>;

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
