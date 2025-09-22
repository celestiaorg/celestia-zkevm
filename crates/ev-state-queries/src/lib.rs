pub mod hyperlane;

use alloy_primitives::FixedBytes;
use alloy_provider::{Provider, fillers::FillProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;

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
}
impl MockStateQueryProvider {
    pub fn new(provider: DefaultProvider) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl StateQueryProvider for MockStateQueryProvider {
    async fn get_state_root(&self, height: u64) -> Result<FixedBytes<32>> {
        let block = self
            .provider
            .get_block(height.into())
            .await?
            .context("Failed to get block")?;
        Ok(block.header.state_root)
    }
    async fn get_height(&self) -> u64 {
        self.provider.get_block_number().await.expect("Failed to get height")
    }
}
