use std::{env, fs, sync::Arc};

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_primitives::FixedBytes;
use alloy_provider::ProviderBuilder;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use celestia_grpc_client::{types::ClientConfig, CelestiaIsmClient, QueryIsmRequest};
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::{
    nmt::{Namespace, NamespaceProof},
    Blob,
};
use ev_types::v1::{
    get_block_request::Identifier, store_service_client::StoreServiceClient, GetBlockRequest, SignedData,
};
use ev_zkevm_types::programs::block::{BlockExecInput, BlockRangeExecOutput, EvCombinedInput};
use eyre::Context;
use eyre::Result;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use sp1_sdk::{include_elf, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use tracing::{debug, info};

use crate::prover::ProgramProver;
use crate::prover::{config::CombinedProverConfig, prover_from_env, SP1Prover};

mod config {
    pub const CELESTIA_RPC_URL: &str = "http://localhost:26658";
    pub const EVM_RPC_URL: &str = "http://localhost:8545";
    pub const SEQUENCER_URL: &str = "http://localhost:7331";
}

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_COMBINED_ELF: &[u8] = include_elf!("ev-combined-program");
pub const ISM_ID: &str = "0x726f757465725f69736d000000000000000000000000002a0000000000000001";
pub const MAX_PROOF_RANGE: u64 = 10;
pub const PARALLELISM: u64 = 1;
pub struct EvCombinedProver {
    config: CombinedProverConfig,
    prover: Arc<SP1Prover>,
}

#[async_trait]
impl ProgramProver for EvCombinedProver {
    type Config = CombinedProverConfig;
    type Input = EvCombinedInput;
    type Output = BlockRangeExecOutput;

    fn cfg(&self) -> &Self::Config {
        &self.config
    }

    fn build_stdin(&self, input: Self::Input) -> AnyhowResult<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);
        Ok(stdin)
    }
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> AnyhowResult<Self::Output> {
        Ok(bincode::deserialize::<BlockRangeExecOutput>(
            proof.public_values.as_slice(),
        )?)
    }

    fn prover(&self) -> Arc<SP1Prover> {
        Arc::clone(&self.prover)
    }
}

impl EvCombinedProver {
    pub fn new() -> Result<Self> {
        let prover = prover_from_env();
        let config = EvCombinedProver::default_config(prover.as_ref());
        Ok(Self { config, prover })
    }

    pub fn default_config(prover: &SP1Prover) -> CombinedProverConfig {
        let (pk, vk) = prover.setup(EV_COMBINED_ELF);
        CombinedProverConfig::new(pk, vk, SP1ProofMode::Groth16)
    }

    pub async fn run(self) -> Result<()> {
        let config = ClientConfig::from_env()?;
        let ism_client = CelestiaIsmClient::new(config).await?;
        let rpc_url = "http://localhost:26658";
        let client = Client::new(rpc_url, None).await?;
        loop {
            let resp = ism_client.ism(QueryIsmRequest { id: ISM_ID.to_string() }).await?;

            let ism = resp.ism.ok_or_else(|| eyre::eyre!("ZKISM not found"))?;
            let trusted_root_hex = alloy::hex::encode(ism.state_root);
            let latest_celestia_header = client.header_local_head().await?;
            let trusted_height = ism.height;
            let trusted_celestia_height = ism.celestia_height;
            let trusted_celestia_header_hash = ism.celestia_header_hash;

            let celestia_start_height = ism.celestia_height + 1;
            let num_blocks = latest_celestia_header.height().value() - celestia_start_height;
            info!("Celestia start height: {}", celestia_start_height);
        }
    }
}

async fn prepare_combined_inputs(
    celestia_client: &Client,
    start_height: u64,
    trusted_height: &mut u64,
    trusted_celestia_height: u64,
    trusted_celestia_header_hash: [u8; 32],
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
) -> Result<SP1Stdin> {
    let genesis_path = dirs::home_dir()
        .ok_or_else(|| eyre::eyre!("cannot find home directory"))?
        .join(".ev-prover")
        .join("config")
        .join("genesis.json");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(
        genesis_path
            .to_str()
            .ok_or_else(|| eyre::eyre!("Invalid genesis path"))?,
    )?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE")?;
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    let pub_key = get_sequencer_pubkey().await?;
    let mut block_inputs: Vec<BlockExecInput> = Vec::new();
    for block_number in start_height..=(start_height + num_blocks) {
        block_inputs.push(
            get_block_inputs(
                &celestia_client,
                block_number,
                namespace,
                *trusted_height,
                *trusted_root,
                chain_spec.clone(),
                genesis.clone(),
                pub_key.clone(),
            )
            .await?,
        );
    }

    // reinitialize the prover client
    let mut stdin = SP1Stdin::new();
    let range_prover_elf = fs::read("elfs/ev-range-exec-elf")?;

    let input = EvCombinedInput {
        blocks: block_inputs,
        trusted_height: *trusted_height,
        trusted_root: *trusted_root,
    };
    stdin.write(&input);
    Ok(stdin)
}

/// Loads the genesis file from disk and converts it into a ChainSpec
fn load_chain_spec_from_genesis(path: &str) -> Result<(Genesis, Arc<ChainSpec>)> {
    let genesis_json = fs::read_to_string(path).with_context(|| format!("Failed to read genesis file at {path}"))?;
    let alloy_genesis: AlloyGenesis = serde_json::from_str(&genesis_json)?;

    let genesis = Genesis::Custom(alloy_genesis.config);
    let chain_spec: Arc<ChainSpec> = Arc::new((&genesis).try_into()?);

    Ok((genesis, chain_spec))
}

async fn get_sequencer_pubkey() -> Result<Vec<u8>> {
    debug!("Connecting to sequencer url: {}", config::SEQUENCER_URL);
    let mut sequencer_client = StoreServiceClient::connect(config::SEQUENCER_URL).await?;
    debug!("Connected to sequencer url: {}", config::SEQUENCER_URL);
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };
    debug!("Getting block from sequencer url: {}", config::SEQUENCER_URL);
    let resp = sequencer_client.get_block(block_req).await?;
    debug!("Got block from sequencer url: {}", config::SEQUENCER_URL);
    let pub_key = resp
        .into_inner()
        .block
        .ok_or_else(|| eyre::eyre!("Block not found"))?
        .header
        .ok_or_else(|| eyre::eyre!("Header not found"))?
        .signer
        .ok_or_else(|| eyre::eyre!("Signer not found"))?
        .pub_key;

    Ok(pub_key[4..].to_vec())
}

#[allow(clippy::too_many_arguments)]
async fn get_block_inputs(
    celestia_client: &Client,
    block_number: u64,
    namespace: Namespace,
    trusted_height: u64,
    trusted_root: FixedBytes<32>,
    chain_spec: Arc<ChainSpec>,
    genesis: Genesis,
    pub_key: Vec<u8>,
) -> Result<BlockExecInput> {
    let blobs: Vec<Blob> = celestia_client
        .blob_get_all(block_number, &[namespace])
        .await?
        .unwrap_or_default();
    debug!("Got {} blobs for block: {}", blobs.len(), block_number);

    let extended_header = celestia_client.header_get_by_height(block_number).await?;
    let namespace_data = celestia_client
        .share_get_namespace_data(&extended_header, namespace)
        .await?;
    let mut proofs: Vec<NamespaceProof> = Vec::new();
    for row in namespace_data.rows {
        proofs.push(row.proof);
    }
    debug!("Got NamespaceProofs, total: {}", proofs.len());

    let mut executor_inputs: Vec<EthClientExecutorInput> = Vec::new();
    for blob in blobs.as_slice() {
        let data = match SignedData::decode(blob.data.as_slice()) {
            Ok(data) => data.data.ok_or_else(|| eyre::eyre!("Data not found"))?,
            Err(_) => continue,
        };
        let height = data.metadata.ok_or_else(|| eyre::eyre!("Metadata not found"))?.height;
        debug!("Got SignedData for EVM block {height}");

        let client_executor_input =
            generate_client_executor_input(config::EVM_RPC_URL, height, chain_spec.clone(), genesis.clone()).await?;
        executor_inputs.push(client_executor_input);
    }

    let input = BlockExecInput {
        header_raw: serde_cbor::to_vec(&extended_header.header)?,
        dah: extended_header.dah,
        blobs_raw: serde_cbor::to_vec(&blobs)?,
        pub_key: pub_key.clone(),
        namespace,
        proofs,
        executor_inputs: executor_inputs.clone(),
        trusted_height,
        trusted_root,
    };

    Ok(input)
}

/// Generates the client executor input (STF) for an EVM block.
async fn generate_client_executor_input(
    rpc_url: &str,
    block_number: u64,
    chain_spec: Arc<ChainSpec>,
    genesis: Genesis,
) -> Result<EthClientExecutorInput> {
    let host_executor = EthHostExecutor::eth(chain_spec.clone(), None);

    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

    let client_input = host_executor
        .execute(block_number, &rpc_db, &provider, genesis, None, false)
        .await
        .wrap_err_with(|| format!("Failed to execute block {block_number}"))?;

    Ok(client_input)
}
