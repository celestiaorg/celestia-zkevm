use std::sync::Arc;

use alloy_provider::ProviderBuilder;
use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::nmt::Namespace;
use celestia_types::{Blob, Commitment, ExtendedHeader, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use reth_chainspec::ChainSpec;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use sha3::{Digest, Keccak256};
use sp1_sdk::include_elf;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

pub struct BlockProver {
    pub evm_rpc_url: String,
    pub celestia_rpc_url: String,
    pub chain_spec: Arc<ChainSpec>,
    pub genesis: Genesis,
    pub namespace: Namespace,
}

impl BlockProver {
    pub async fn new(
        evm_rpc_url: String,
        celestia_rpc_url: String,
        chain_spec: Arc<ChainSpec>,
        genesis: Genesis,
        namespace: Namespace,
    ) -> Result<Self> {
        Ok(Self {
            evm_rpc_url,
            celestia_rpc_url,
            chain_spec,
            genesis,
            namespace,
        })
    }

    pub async fn generate_stf(&self, block_number: u64) -> Result<Vec<u8>> {
        let host_executor = EthHostExecutor::eth(self.chain_spec.clone(), None);
        let provider = ProviderBuilder::new().on_http(self.evm_rpc_url.parse()?);
        let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

        let client_input = host_executor
            .execute(block_number, &rpc_db, &provider, self.genesis.clone(), None, false)
            .await?;

        Ok(bincode::serialize(&client_input)?)
    }

    pub async fn inclusion_height(&self, _block_number: u64) -> Result<(u64, Commitment)> {
        panic!("TODO(unimplemented): Query rollkit rpc for DA inclusion height");
    }

    pub async fn blob_inclusion_proof(
        &self,
        inclusion_height: u64,
        commitment: Commitment,
    ) -> Result<(KeccakInclusionToDataRootProofInput, ExtendedHeader)> {
        let celestia_client = Client::new(&self.celestia_rpc_url, None).await?;

        let blob = celestia_client
            .blob_get(inclusion_height, self.namespace, commitment)
            .await?;
        let header = celestia_client.header_get_by_height(inclusion_height).await?;
        let share_proof = self.verify_blob_inclusion(&header, &blob).await?;

        let keccak_hash: [u8; 32] = Keccak256::new().chain_update(&blob.data).finalize().into();

        let input = KeccakInclusionToDataRootProofInput {
            data: blob.data.clone(),
            namespace_id: self.namespace,
            share_proofs: share_proof.share_proofs.clone(),
            row_proof: share_proof.row_proof.clone(),
            data_root: header.dah.hash().as_bytes().try_into()?,
            keccak_hash,
        };

        Ok((input, header))
    }

    async fn verify_blob_inclusion(&self, header: &ExtendedHeader, blob: &Blob) -> Result<ShareProof> {
        let eds_size = header.dah.row_roots().len() as u64;
        let ods_size = eds_size / 2;
        let first_row_index = blob.index.unwrap() / eds_size;
        let ods_index = blob.index.unwrap() - (first_row_index * ods_size);

        let mut header = header.clone();
        header.header.version.app = 3;

        let celestia_client = Client::new(&self.celestia_rpc_url, None).await?;
        let range_response = celestia_client
            .share_get_range(&header, ods_index, ods_index + blob.shares_len() as u64)
            .await?;

        let share_proof = range_response.proof;
        share_proof.verify(header.dah.hash())?;
        Ok(share_proof)
    }
}
