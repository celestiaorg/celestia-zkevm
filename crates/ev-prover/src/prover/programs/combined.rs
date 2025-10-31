use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{
    generate_client_executor_input,
    prover::{
        config::{BATCH_SIZE, MIN_BATCH_SIZE, WARN_DISTANCE},
        MessageProofRequest, MessageProofSync, ProverConfig, RangeProofCommitted,
    },
};
use alloy_primitives::FixedBytes;
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use celestia_grpc_client::{CelestiaIsmClient, MsgUpdateZkExecutionIsm, QueryIsmRequest};
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::{
    nmt::{Namespace, NamespaceProof},
    Blob,
};
use ev_types::v1::SignedData;
use ev_zkevm_types::programs::block::{BlockExecInput, BlockRangeExecOutput, EvCombinedInput};
use hex::decode;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_primitives::genesis::Genesis;
use sp1_sdk::{include_elf, SP1ProofMode, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use tokio::{sync::mpsc, time::interval};
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::prover::ProgramProver;
use crate::prover::{prover_from_env, SP1Prover};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_COMBINED_ELF: &[u8] = include_elf!("ev-combined-program");

struct ProverStatus {
    trusted_height: u64,
    trusted_root: FixedBytes<32>,
    trusted_celestia_height: u64,
    celestia_head: u64,
}

impl ProverStatus {
    fn has_required_batch(&self, batch_size: u64) -> bool {
        self.trusted_celestia_height + batch_size <= self.celestia_head
    }

    fn blocks_needed(&self, batch_size: u64) -> u64 {
        (self.trusted_celestia_height + batch_size).saturating_sub(self.celestia_head)
    }

    fn distance(&self) -> u64 {
        self.celestia_head.saturating_sub(self.trusted_celestia_height)
    }
}

pub struct AppContext {
    pub celestia_client: Arc<Client>,
    pub evm_rpc: String,
    pub ism_client: Arc<CelestiaIsmClient>,
    pub chain_spec: Arc<ChainSpec>,
    pub genesis: Genesis,
    pub namespace: Namespace,
    pub pub_key: Arc<Vec<u8>>,
}
impl AppContext {
    pub async fn from_config(config: &Config, ism_client: Arc<CelestiaIsmClient>) -> Result<Self> {
        let celestia_client = Client::new(&config.rpc.celestia_rpc, None).await?;
        let pub_key_hex = config.pub_key.trim_start_matches("0x");
        let sequencer_pubkey = Arc::new(decode(pub_key_hex)?);
        let genesis = Config::load_genesis()?;
        let chain_spec: Arc<ChainSpec> = Arc::new(
            (&genesis)
                .try_into()
                .map_err(|e| anyhow!("Failed to convert genesis to chain spec: {e}"))?,
        );

        Ok(Self {
            celestia_client: Arc::new(celestia_client),
            evm_rpc: config.rpc.evreth_rpc.clone(),
            ism_client,
            chain_spec,
            genesis,
            namespace: config.namespace,
            pub_key: sequencer_pubkey,
        })
    }

    pub fn load_genesis_and_chainspec() -> Result<(Genesis, Arc<ChainSpec>)> {
        let genesis = Config::load_genesis()?;
        let chain_spec: Arc<ChainSpec> = Arc::new(
            (&genesis)
                .try_into()
                .map_err(|e| anyhow!("Failed to convert genesis to chain spec: {e}"))?,
        );

        Ok((genesis, chain_spec))
    }
}

#[derive(Clone)]
pub struct CombinedProverConfig {
    pub pk: Arc<SP1ProvingKey>,
    pub vk: Arc<SP1VerifyingKey>,
    pub proof_mode: SP1ProofMode,
}

impl CombinedProverConfig {
    pub fn new(pk: SP1ProvingKey, vk: SP1VerifyingKey, mode: SP1ProofMode) -> Self {
        CombinedProverConfig {
            pk: Arc::new(pk),
            vk: Arc::new(vk),
            proof_mode: mode,
        }
    }
}

impl ProverConfig for CombinedProverConfig {
    fn pk(&self) -> Arc<SP1ProvingKey> {
        Arc::clone(&self.pk)
    }

    fn vk(&self) -> Arc<SP1VerifyingKey> {
        Arc::clone(&self.vk)
    }

    fn proof_mode(&self) -> SP1ProofMode {
        self.proof_mode
    }
}

pub struct EvCombinedProver {
    app: AppContext,
    range_tx: mpsc::Sender<MessageProofRequest>,
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
    pub fn new(app: AppContext, range_tx: mpsc::Sender<MessageProofRequest>) -> Result<Self> {
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

    pub async fn run(self, message_sync: Arc<MessageProofSync>) -> Result<()> {
        let mut batch_size = BATCH_SIZE;
        let mut scan_head: Option<u64> = None;
        let mut poll = interval(Duration::from_secs(6)); // BlockTime=6s

        loop {
            message_sync.wait_for_idle().await;
            poll.tick().await;

            let status = self.load_prover_status().await?;

            if scan_head.is_none() {
                scan_head = Some(status.trusted_celestia_height + 1);
            }

            let scan_start = scan_head.unwrap();
            if scan_start < status.celestia_head {
                batch_size = self
                    .calculate_batch_size(
                        scan_start,
                        status.celestia_head,
                        status.trusted_celestia_height,
                        batch_size,
                    )
                    .await?;
            }

            if !status.has_required_batch(batch_size) {
                let blocks_needed = status.blocks_needed(batch_size);
                let current_height = status.celestia_head;
                debug!("Waiting for {blocks_needed} more blocks to reach required batch size. Current height: {current_height}");
                continue;
            }

            let distance = status.distance();
            if distance >= WARN_DISTANCE {
                warn!("Prover is {distance} blocks behind Celestia head");
            } else {
                info!("Prover is {distance} blocks behind Celestia head");
            }

            let start_height = status.trusted_celestia_height + 1;
            let stdin = self.build_proof_inputs(start_height, &status, batch_size).await?;

            let start_time = Instant::now();
            // TODO: we can use the self.prove() method to get (proof, outputs)
            let proof = self
                .prover
                .prove(&self.config.pk, &stdin, SP1ProofMode::Groth16)
                .context("Failed to prove")?;
            info!("Proof generation time: {}", start_time.elapsed().as_millis());

            let block_proof_msg = MsgUpdateZkExecutionIsm::new(
                self.app.ism_client.ism_id().to_string(),
                proof.bytes(),
                proof.public_values.as_slice().to_vec(),
                self.app.ism_client.signer_address().to_string(),
            );

            info!("Updating ZKISM on Celestia...");
            let response = self.app.ism_client.send_tx(block_proof_msg).await?;
            assert!(response.success);
            info!("[Done] ZKISM was updated successfully");

            let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;

            // reset batch size and fast forward checkpoints
            batch_size = BATCH_SIZE;
            scan_head = Some(status.celestia_head + 1);

            let permit = message_sync.begin().await;
            let commit = RangeProofCommitted::new(public_values.new_height, public_values.new_state_root);
            let request = MessageProofRequest::with_permit(commit, permit);
            self.range_tx.send(request).await?;
        }
    }

    async fn load_prover_status(&self) -> Result<ProverStatus> {
        let resp = self
            .app
            .ism_client
            .ism(QueryIsmRequest {
                id: self.app.ism_client.ism_id().to_string(),
            })
            .await?;
        let ism = resp.ism.ok_or_else(|| anyhow!("ZKISM not found"))?;
        let trusted_root = FixedBytes::from_slice(&ism.state_root);
        let celestia_head = self.app.celestia_client.header_local_head().await?.height().value();

        Ok(ProverStatus {
            trusted_height: ism.height,
            trusted_root,
            trusted_celestia_height: ism.celestia_height,
            celestia_head,
        })
    }

    async fn calculate_batch_size(
        &self,
        scan_start: u64,
        latest_head: u64,
        trusted_celestia_height: u64,
        current_batch: u64,
    ) -> Result<u64> {
        if scan_start >= latest_head {
            return Ok(current_batch);
        }

        let namespace = self.app.namespace;
        for height in scan_start..latest_head {
            if !self.is_empty_block(height, namespace).await? {
                // Ensure batch size stays within allowed range
                let blocks_elapsed = height.saturating_sub(trusted_celestia_height);
                let batch_size = blocks_elapsed.clamp(MIN_BATCH_SIZE, BATCH_SIZE);
                debug!("Found non-empty block at height {height}, adjusting batch size to {batch_size}");
                return Ok(batch_size);
            }
        }

        Ok(BATCH_SIZE)
    }

    async fn is_empty_block(&self, height: u64, namespace: Namespace) -> Result<bool> {
        let blobs: Vec<Blob> = self
            .app
            .celestia_client
            .blob_get_all(height, &[namespace])
            .await?
            .unwrap_or_default();

        Ok(blobs.is_empty())
    }

    async fn build_proof_inputs(&self, start_height: u64, status: &ProverStatus, batch_size: u64) -> Result<SP1Stdin> {
        let mut current_height = status.trusted_height;
        let mut current_root = status.trusted_root;

        let namespace = self.app.namespace;
        let end_height = start_height + batch_size - 1;

        let mut block_inputs: Vec<BlockExecInput> = Vec::new();
        for block_number in start_height..=end_height {
            let input = self
                .build_block_input(
                    block_number,
                    namespace,
                    &mut current_height,
                    &mut current_root,
                    self.app.chain_spec.clone(),
                    self.app.genesis.clone(),
                )
                .await?;

            block_inputs.push(input);
        }

        let mut stdin = SP1Stdin::new();
        stdin.write(&EvCombinedInput { blocks: block_inputs });
        Ok(stdin)
    }

    async fn build_block_input(
        &self,
        block_number: u64,
        namespace: Namespace,
        trusted_height: &mut u64,
        trusted_root: &mut FixedBytes<32>,
        chain_spec: Arc<ChainSpec>,
        genesis: Genesis,
    ) -> Result<BlockExecInput> {
        let blobs: Vec<Blob> = self
            .app
            .celestia_client
            .blob_get_all(block_number, &[namespace])
            .await?
            .unwrap_or_default();
        debug!("Got {} blobs for block: {}", blobs.len(), block_number);

        let extended_header = self.app.celestia_client.header_get_by_height(block_number).await?;
        let namespace_data = self
            .app
            .celestia_client
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
                pub_key: self.app.pub_key.to_vec(),
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
            let data = signed_data.data.ok_or_else(|| anyhow!("Data not found"))?;
            let height = data.metadata.ok_or_else(|| anyhow!("Metadata not found"))?.height;
            last_height = height;
            debug!("Got SignedData for EVM block {height}");

            let client_executor_input =
                generate_client_executor_input(&self.app.evm_rpc, height, chain_spec.clone(), genesis.clone()).await?;
            executor_inputs.push(client_executor_input);
        }

        let input = BlockExecInput {
            header_raw: serde_cbor::to_vec(&extended_header.header)?,
            dah: extended_header.dah,
            blobs_raw: serde_cbor::to_vec(&blobs)?,
            pub_key: self.app.pub_key.to_vec(),
            namespace,
            proofs,
            executor_inputs: executor_inputs.clone(),
            trusted_height: *trusted_height,
            trusted_root: *trusted_root,
        };

        let provider = ProviderBuilder::new().connect_http(self.app.evm_rpc.parse()?);
        let block = provider
            .get_block_by_number(last_height.into())
            .await?
            .ok_or_else(|| anyhow!("Block {last_height} not found"))?;

        *trusted_height = last_height;
        *trusted_root = block.header.state_root;
        debug!(
            "Updated trusted_height to {} and trusted_root to {:?}",
            trusted_height, trusted_root
        );

        Ok(input)
    }
}
