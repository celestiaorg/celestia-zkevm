use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    generate_client_executor_input, get_sequencer_pubkey, load_chain_spec_from_genesis, prover::RangeProofCommitted,
    ISM_ID,
};
use alloy::hex::FromHex;
use alloy_primitives::FixedBytes;
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::{Context, Result};
use async_trait::async_trait;
use celestia_grpc_client::{CelestiaIsmClient, MsgUpdateZkExecutionIsm, QueryIsmRequest};
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::{
    nmt::{Namespace, NamespaceProof},
    Blob,
};
use ev_types::v1::SignedData;
use ev_zkevm_types::programs::block::{BlockExecInput, BlockRangeExecOutput, EvCombinedInput};
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_primitives::genesis::Genesis;
use sp1_sdk::{include_elf, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::prover::ProgramProver;
use crate::prover::{config::CombinedProverConfig, prover_from_env, SP1Prover};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_COMBINED_ELF: &[u8] = include_elf!("ev-combined-program");
pub const BATCH_SIZE: u64 = 50;
pub const WARN_DISTANCE: u64 = 60;
pub const ERR_DISTANCE: u64 = 120;

pub struct AppContext {
    // reth http, for example http://127.0.0.1:8545
    pub evm_rpc: String,
    // celestia rpc, for example http://127.0.0.1:26658
    pub celestia_rpc: String,
}
impl AppContext {
    pub fn new(evm_rpc: String, celestia_rpc: String) -> Self {
        Self { evm_rpc, celestia_rpc }
    }
}
impl Default for AppContext {
    fn default() -> Self {
        Self::new(
            "http://127.0.0.1:8545".to_string(),
            "http://127.0.0.1:26658".to_string(),
        )
    }
}

pub struct EvCombinedProver {
    app: AppContext,
    range_tx: tokio::sync::mpsc::Sender<RangeProofCommitted>,
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

    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);
        Ok(stdin)
    }
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<BlockRangeExecOutput>(
            proof.public_values.as_slice(),
        )?)
    }

    fn prover(&self) -> Arc<SP1Prover> {
        Arc::clone(&self.prover)
    }
}

impl EvCombinedProver {
    pub fn new(app: AppContext, range_tx: tokio::sync::mpsc::Sender<RangeProofCommitted>) -> Result<Self> {
        let prover = prover_from_env();
        let config = EvCombinedProver::default_config(prover.as_ref());

        Ok(Self {
            app,
            config,
            prover,
            range_tx,
        })
    }

    pub fn default_config(prover: &SP1Prover) -> CombinedProverConfig {
        let (pk, vk) = prover.setup(EV_COMBINED_ELF);
        CombinedProverConfig::new(pk, vk, SP1ProofMode::Groth16)
    }

    pub async fn run(self, ism_client: Arc<CelestiaIsmClient>) -> Result<()> {
        let client = Client::new(&self.app.celestia_rpc, None).await?;

        let mut known_celestia_height: u64 = 0;

        loop {
            let resp = ism_client.ism(QueryIsmRequest { id: ISM_ID.to_string() }).await?;
            let ism = resp.ism.ok_or_else(|| anyhow::anyhow!("ZKISM not found"))?;
            let trusted_root_hex = alloy::hex::encode(ism.state_root);
            let latest_celestia_header = client.header_local_head().await?;
            let mut trusted_height = ism.height;
            let mut trusted_root = FixedBytes::from_hex(&trusted_root_hex)?;
            let trusted_celestia_height = ism.celestia_height;
            let latest_celestia_height = latest_celestia_header.height().value();
            if latest_celestia_height == known_celestia_height {
                info!("Celestia height has not changed, waiting for 1 second");
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            if trusted_celestia_height + BATCH_SIZE > latest_celestia_height {
                let blocks_needed = (trusted_celestia_height + BATCH_SIZE).saturating_sub(latest_celestia_height);
                warn!("Waiting for {blocks_needed} more blocks to reach required batch size");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let distance = latest_celestia_height.saturating_sub(trusted_celestia_height);

            if distance >= ERR_DISTANCE {
                error!("Prover is {distance} blocks behind Celestia head");
            } else if distance >= WARN_DISTANCE {
                warn!("Prover is {distance} blocks behind Celestia head");
            } else {
                info!("Prover is {distance} blocks behind Celestia head");
            }

            let celestia_start_height = ism.celestia_height + 1;
            let stdin = prepare_combined_inputs(
                &client,
                &self.app.evm_rpc,
                celestia_start_height,
                &mut trusted_height,
                BATCH_SIZE,
                &mut trusted_root,
            )
            .await?;

            let start_time = Instant::now();
            let proof = self
                .prover
                .prove(&self.config.pk, &stdin, SP1ProofMode::Groth16)
                .context("Failed to prove")?;
            info!("Proof generation time: {}", start_time.elapsed().as_millis());

            let block_proof_msg = MsgUpdateZkExecutionIsm::new(
                ISM_ID.to_string(),
                proof.bytes(),
                proof.public_values.as_slice().to_vec(),
                ism_client.signer_address().to_string(),
            );
            info!("Updating ZKISM on Celestia...");
            let response = ism_client.send_tx(block_proof_msg).await?;
            assert!(response.success);
            info!("[Done] ZKISM was updated successfully");
            let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
            known_celestia_height = public_values.celestia_height;
            // use shared channel to request message proof for new height and root
            self.range_tx
                .send(RangeProofCommitted {
                    trusted_height: public_values.new_height,
                    trusted_root: public_values.new_state_root,
                })
                .await?;
        }
    }
}

async fn prepare_combined_inputs(
    celestia_client: &Client,
    evm_rpc: &str,
    start_height: u64,
    trusted_height: &mut u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
) -> Result<SP1Stdin> {
    let genesis_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot find home directory"))?
        .join(".ev-prover")
        .join("config")
        .join("genesis.json");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(
        genesis_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid genesis path"))?,
    )?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE")?;
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    let pub_key = get_sequencer_pubkey("http://localhost:7331".to_string()).await?;
    let mut block_inputs: Vec<BlockExecInput> = Vec::new();
    for block_number in start_height..=(start_height + num_blocks) {
        block_inputs.push(
            get_block_inputs(
                celestia_client,
                evm_rpc,
                block_number,
                namespace,
                trusted_height,
                trusted_root,
                chain_spec.clone(),
                genesis.clone(),
                pub_key.clone(),
            )
            .await?,
        );
    }

    // reinitialize the prover client
    let mut stdin = SP1Stdin::new();
    let input = EvCombinedInput { blocks: block_inputs };
    stdin.write(&input);
    Ok(stdin)
}

#[allow(clippy::too_many_arguments)]
pub async fn get_block_inputs(
    celestia_client: &Client,
    evm_rpc: &str,
    block_number: u64,
    namespace: Namespace,
    trusted_height: &mut u64,
    trusted_root: &mut FixedBytes<32>,
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
    if blobs.is_empty() {
        debug!(
            "No blobs for Celestia height {}, keeping trusted_height={} and trusted_root unchanged",
            block_number, trusted_height
        );
        return Ok(BlockExecInput {
            header_raw: serde_cbor::to_vec(&extended_header.header)?,
            dah: extended_header.dah,
            blobs_raw: serde_cbor::to_vec(&blobs)?,
            pub_key: pub_key.clone(),
            namespace,
            proofs,
            executor_inputs: vec![],
            trusted_height: *trusted_height,
            trusted_root: *trusted_root,
        });
    }

    let mut last_height = 0;
    for blob in blobs.as_slice() {
        let signed_data = match SignedData::decode(blob.data.as_slice()) {
            Ok(data) => data,
            Err(_) => continue,
        };
        let data = signed_data.data.ok_or_else(|| anyhow::anyhow!("Data not found"))?;
        let height = data
            .metadata
            .ok_or_else(|| anyhow::anyhow!("Metadata not found"))?
            .height;
        last_height = height;
        debug!("Got SignedData for EVM block {height}");

        let client_executor_input =
            generate_client_executor_input(evm_rpc, height, chain_spec.clone(), genesis.clone()).await?;
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
        trusted_height: *trusted_height,
        trusted_root: *trusted_root,
    };

    let provider = ProviderBuilder::new().connect_http(evm_rpc.parse()?);
    let block = provider
        .get_block_by_number(last_height.into())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Block {} not found", last_height))?;

    *trusted_height = last_height;
    *trusted_root = block.header.state_root;
    debug!(
        "Updated trusted_height to {} and trusted_root to {:?}",
        trusted_height, trusted_root
    );

    Ok(input)
}
