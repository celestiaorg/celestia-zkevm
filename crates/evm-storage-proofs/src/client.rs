use alloy::{providers::Provider, rpc::types::EIP1186AccountProofResponse};
use alloy_primitives::{Address, FixedBytes, hex::FromHex};
use anyhow::{Context, Result};

pub type DefaultProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        alloy::providers::Identity,
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::GasFiller,
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::BlobGasFiller,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::NonceFiller,
                    alloy::providers::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy::providers::RootProvider,
>;

pub struct EvmClient {
    pub provider: DefaultProvider,
}

impl EvmClient {
    pub fn new(provider: DefaultProvider) -> Self {
        Self { provider }
    }

    // Get a Patricia Trie merkle proof for some key under some account for a given block height.
    pub async fn get_proof(
        &self,
        keys: &[&str],
        contract: Address,
        height: Option<u64>,
    ) -> Result<EIP1186AccountProofResponse> {
        if height.is_some() {
            let proof: EIP1186AccountProofResponse = self
                .provider
                .get_proof(
                    contract,
                    keys.iter().map(|k| FixedBytes::from_hex(k).unwrap()).collect(),
                )
                .block_id(height.expect("height must be provided").into())
                .await?;
            Ok(proof)
        } else {
            let proof: EIP1186AccountProofResponse = self
                .provider
                .get_proof(
                    contract,
                    keys.iter().map(|k| FixedBytes::from_hex(k).unwrap()).collect(),
                )
                .await?;
            Ok(proof)
        }
    }

    pub async fn get_state_root(&self, height: u64) -> Result<String> {
        let block = self
            .provider
            .get_block(height.into())
            .await?
            .context("Failed to get block")?;
        Ok(alloy::hex::encode(block.header.state_root.0))
    }
}

#[cfg(feature = "alloy")]
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{
        client::{DefaultProvider, EvmClient},
        types::{HYPERLANE_MERKLE_TREE_KEYS, HyperlaneBranchProof},
    };
    use alloy::{
        hex::FromHex,
        providers::{Provider, ProviderBuilder},
        transports::http::reqwest::Url,
    };
    use alloy_primitives::Address;

    #[tokio::test]
    async fn test_single_node_hyperlane_tree_branch() {
        let contract = Address::from_hex("0xfcb1d485ef46344029d9e8a7925925e146b3430e").unwrap();
        let provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str("http://127.0.0.1:8545").unwrap());
        let height = provider.get_block_number().await.unwrap();
        let client = EvmClient::new(provider);
        let key = "0x0000000000000000000000000000000000000000000000000000000000000097";

        let proof = client
            .get_proof(
                // starts at 151 up to 182, count is located at 183
                // get the first one to check against off-chain tree
                &[key],
                contract,
                Some(height),
            )
            .await
            .unwrap();

        let execution_state_root = client.get_state_root(height).await.unwrap();
        let branch_proof = HyperlaneBranchProof::new(proof);
        let verified = branch_proof.verify_single(key, contract, &execution_state_root);
        assert!(verified.unwrap());
    }

    #[tokio::test]
    async fn test_all_nodes_hyperlane_tree_branch() {
        let contract = Address::from_hex("0xfcb1d485ef46344029d9e8a7925925e146b3430e").unwrap();
        let provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str("http://127.0.0.1:8545").unwrap());
        let height = provider.get_block_number().await.unwrap();
        let client = EvmClient::new(provider);

        let proof = client
            .get_proof(
                // starts at 151 up to 182, count is located at 183
                // get the first one to check against off-chain tree
                &HYPERLANE_MERKLE_TREE_KEYS,
                contract,
                Some(height),
            )
            .await
            .unwrap();

        let execution_state_root = client.get_state_root(height).await.unwrap();
        let branch_proof = HyperlaneBranchProof::new(proof);
        let verified = branch_proof.verify(&HYPERLANE_MERKLE_TREE_KEYS, contract, &execution_state_root);
        assert!(verified.unwrap());

        for (idx, key) in HYPERLANE_MERKLE_TREE_KEYS.iter().enumerate() {
            let value = branch_proof.get_branch_node(idx).unwrap();
            println!("key: {key}, value: {value}");
        }
    }
}
