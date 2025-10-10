#![allow(dead_code)]
use celestia_types::ExtendedHeader;
use std::collections::BTreeMap;
use std::fs;
use std::result::Result::{Err, Ok};
use std::sync::Arc;

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_primitives::FixedBytes;
use alloy_provider::ProviderBuilder;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use celestia_rpc::blob::BlobsAtHeight;
use celestia_rpc::{client::Client, BlobClient, HeaderClient, ShareClient};
use celestia_types::nmt::{Namespace, NamespaceProof};
use celestia_types::Blob;
use ev_types::v1::SignedData;
use ev_zkevm_types::programs::block::{BlockExecInput, BlockExecOutput};
use jsonrpsee_core::client::Subscription;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use sp1_sdk::{include_elf, EnvProver, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use tokio::{
    sync::{mpsc, mpsc::Sender, RwLock, Semaphore},
    task::JoinSet,
};
use tracing::{debug, error, info};

use crate::config::config::{Config, APP_HOME, CONFIG_DIR, GENESIS_FILE};
use crate::prover::{ProgramProver, ProofCommitted, ProverConfig};
use storage::proofs::ProofStorage;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_EXEC_ELF: &[u8] = include_elf!("ev-exec-program");

/// AppContext encapsulates the full set of RPC endpoints and configuration
/// needed to fetch input data for execution and data availability proofs.
///
/// This separates RPC concerns from the proving logic, allowing `AppContext`
/// to be responsible for gathering the data required for the proof system inputs.
pub struct AppContext {
    pub chain_spec: Arc<ChainSpec>,
    pub genesis: Genesis,
    pub namespace: Namespace,
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub pub_key: Vec<u8>,
    pub trusted_state: RwLock<TrustedState>,
}

/// TrustedState tracks the trusted height and state root which is provided to the proof system as inputs.
/// This type is wrapped in a RwLock by the AppContext such that it can be updated safely across concurrent tasks.
/// Updates are made optimisticly using the EthClientExecutorInputs queried from the configured EVM full node.
pub struct TrustedState {
    height: u64,
    root: FixedBytes<32>,
}

impl TrustedState {
    pub fn new(height: u64, root: FixedBytes<32>) -> Self {
        Self { height, root }
    }
}

impl AppContext {
    pub fn from_config(config: Config) -> Result<Self> {
        let genesis = AppContext::load_genesis().context("Error loading app genesis")?;
        let chain_spec: Arc<ChainSpec> = Arc::new(
            (&genesis)
                .try_into()
                .map_err(|e| anyhow!("Failed to convert genesis to chain spec: {e}"))?,
        );

        let raw_ns = hex::decode(config.namespace_hex)?;
        let namespace = Namespace::new_v0(raw_ns.as_ref()).context("Failed to construct Namespace")?;
        let pub_key = hex::decode(config.pub_key)?;
        let trusted_state = RwLock::new(TrustedState::new(0, chain_spec.genesis_header().state_root));

        Ok(AppContext {
            chain_spec,
            genesis,
            namespace,
            celestia_rpc: config.celestia_rpc,
            evm_rpc: config.evm_rpc,
            pub_key,
            trusted_state,
        })
    }

    fn load_genesis() -> Result<Genesis> {
        let path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join(CONFIG_DIR)
            .join(GENESIS_FILE);

        let raw_genesis = fs::read_to_string(path).context("Failed to read genesis file from path")?;
        let alloy_genesis: AlloyGenesis = serde_json::from_str(&raw_genesis)?;

        let genesis = Genesis::Custom(alloy_genesis.config);
        Ok(genesis)
    }
}

/// A prover for generating SP1 proofs for EVM block execution and data availability in Celestia.
///
/// This struct is responsible for preparing the standard input (`SP1Stdin`)
/// for a zkVM program that takes a blob inclusion proof, data root proof, Celestia Header and
/// EVM state transition function.
pub struct BlockExecProver {
    pub app: AppContext,
    pub config: ProverConfig,
    pub prover: EnvProver,
    pub tx: Sender<ProofCommitted>,
    pub storage: Arc<dyn ProofStorage>,
}

#[async_trait]
impl ProgramProver for BlockExecProver {
    type Input = BlockExecInput;
    type Output = BlockExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &ProverConfig {
        &self.config
    }

    /// Constructs the `SP1Stdin` input required for proving.
    ///
    /// This function serializes and writes structured input data into the
    /// stdin buffer in the expected format for the SP1 program.
    ///
    /// # Errors
    /// Returns an error if serialization of any input component fails.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);
        Ok(stdin)
    }

    /// Parses the `SP1PublicValues` from the proof and converts it into the
    /// program's custom output type.
    ///
    /// # Errors
    /// - Returns an error if deserialization fails.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<BlockExecOutput>(proof.public_values.as_slice())?)
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

// TODO: Add these as fields to the BlockExecProver to make configurable?
const QUEUE_CAP: usize = 256;
const CONCURRENCY: usize = 16;

struct BlockEvent {
    height: u64,
    blobs: Vec<Blob>,
}

impl BlockEvent {
    fn new(height: u64, blobs: Vec<Blob>) -> Self {
        Self { height, blobs }
    }
}

struct ProofJob {
    height: u64,
    extended_header: ExtendedHeader,
    proofs: Vec<NamespaceProof>,
    blobs: Vec<Blob>,
    executor_inputs: Vec<EthClientExecutorInput>,
}

struct ScheduledProofJob {
    job: Arc<ProofJob>,
    trusted_height: u64,
    trusted_root: FixedBytes<32>,
}

impl BlockExecProver {
    /// Creates a new instance of [`BlockExecProver`] for the provided [`AppContext`] using default configuration
    /// and prover environment settings.
    pub fn new(app: AppContext, tx: Sender<ProofCommitted>, storage: Arc<dyn ProofStorage>) -> Result<Arc<Self>> {
        let config = BlockExecProver::default_config();
        let prover = ProverClient::from_env();

        Ok(Arc::new(Self {
            app,
            config,
            prover,
            tx,
            storage,
        }))
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EV_EXEC_ELF,
            proof_mode: SP1ProofMode::Compressed,
        }
    }

    async fn connect_and_subscribe(&self) -> Result<(Arc<Client>, Subscription<BlobsAtHeight>)> {
        let addr = format!("ws://{}", self.app.celestia_rpc);
        let client = Arc::new(Client::new(&addr, None).await.context("celestia ws connect")?);
        let subscription = client
            .blob_subscribe(self.app.namespace)
            .await
            .context("Blob subscription failed")?;

        Ok((client, subscription))
    }

    /// Generates the state transition function (STF) input for a given EVM block number.
    async fn eth_client_executor_input(&self, block_number: u64) -> Result<EthClientExecutorInput> {
        let host_executor = EthHostExecutor::eth(self.app.chain_spec.clone(), None);
        let provider = ProviderBuilder::new().connect_http(self.app.evm_rpc.parse()?);
        let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

        let executor_input = host_executor
            .execute(block_number, &rpc_db, &provider, self.app.genesis.clone(), None, false)
            .await?;

        Ok(executor_input)
    }

    /// Runs the block prover loop with a 3-stage pipeline:
    ///
    /// 1. **Prepare**: For each [`BlockEvent`] received from the Celestia subscription,
    ///    fetch and build the proof inputs in parallel (bounded by `CONCURRENCY`).
    /// 2. **Schedule**: In height order, attach the current trusted snapshot and
    ///    optimistically advance the shared [`TrustedState`] for subsequent jobs.
    /// 3. **Prove**: Spawn proof workers (also concurrency-limited) that generate
    ///    proofs using the assigned inputs.
    ///
    /// The Celestia node produces a new [`BlobsAtHeight`] event for each block. An
    /// event may or may not contain any blobs in the configured namespace at a given
    /// height. Events are fed into the pipeline via the WebSocket subscription, and
    /// proofs are generated concurrently while ensuring the trusted state is updated
    /// monotonically in block-height order.
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (client, mut subscription) = self.connect_and_subscribe().await?;

        // Queues for the 3-stage pipeline
        let (event_tx, mut event_rx) = mpsc::channel::<BlockEvent>(QUEUE_CAP);
        let (job_tx, mut job_rx) = mpsc::channel::<ProofJob>(QUEUE_CAP);
        let (sched_tx, mut sched_rx) = mpsc::channel::<ScheduledProofJob>(QUEUE_CAP);

        // Stage 1: Prepare proof inputs (parallel, IO-bound)
        let sem = Arc::new(Semaphore::new(CONCURRENCY));
        tokio::spawn({
            let client = client.clone();
            let prover = self.clone();

            let job_tx = job_tx.clone();
            let sem = sem.clone();
            async move {
                let mut tasks = JoinSet::new();
                while let Some(event) = event_rx.recv().await {
                    debug!("\nNew block event height={}, blobs={}", event.height, event.blobs.len());
                    let client = client.clone();
                    let prover = prover.clone();

                    let job_tx = job_tx.clone();
                    let permit = sem.clone().acquire_owned().await.unwrap();

                    tasks.spawn(async move {
                        let _permit = permit; // limit concurrent prepares
                        match prover.prepare_inputs(client, event).await {
                            Ok(job) => {
                                let _ = job_tx.send(job).await;
                            }
                            Err(e) => error!("failed to retrieve proof inputs: {e:#}"),
                        }
                    });
                }

                while tasks.join_next().await.is_some() {}
                error!("prepare stage shutting down");
            }
        });

        // Stage 2: Assign the trusted height and root for the next proof (single writer of trusted_state, in height order)
        tokio::spawn({
            let prover = self.clone();
            let sched_tx = sched_tx.clone();
            async move {
                let mut buf: BTreeMap<u64, ProofJob> = BTreeMap::new();
                let mut next_height: Option<u64> = None;

                while let Some(job) = job_rx.recv().await {
                    buf.insert(job.height, job);

                    loop {
                        let height = match next_height {
                            Some(h) => h,
                            None => {
                                if let Some((&min_height, _)) = buf.iter().next() {
                                    next_height = Some(min_height);
                                    min_height
                                } else {
                                    break;
                                }
                            }
                        };

                        let Some(job) = buf.remove(&height) else { break };

                        // Snapshot current trusted state for proof
                        let (trusted_height, trusted_root) = {
                            let s = prover.app.trusted_state.read().await;
                            (s.height, s.root)
                        };

                        // Optimistically advance global trusted_state monotonically for FUTURE jobs
                        if let Some(next) = job.executor_inputs.last() {
                            let mut s = prover.app.trusted_state.write().await;
                            if next.current_block.number > s.height {
                                s.height = next.current_block.number;
                                s.root = next.current_block.state_root;
                            }
                        }

                        let scheduled = ScheduledProofJob {
                            job: Arc::new(job),
                            trusted_height,
                            trusted_root,
                        };

                        if sched_tx.send(scheduled).await.is_err() {
                            break;
                        }

                        next_height = Some(height + 1);
                    }
                }

                error!("schedule stage shutting down");
            }
        });

        // Stage 3: Prove (parallel, CPU/IO-bound for remote prover network)
        let prove_sem = Arc::new(Semaphore::new(CONCURRENCY));
        tokio::spawn({
            let prover = self.clone();
            let prove_sem = prove_sem.clone();

            async move {
                let mut tasks = JoinSet::new();
                while let Some(scheduled) = sched_rx.recv().await {
                    let prover = prover.clone();
                    let permit = prove_sem.clone().acquire_owned().await.unwrap();
                    tasks.spawn(async move {
                        let _permit = permit; // limit concurrent proofs

                        if let Err(e) = prover.prove_and_store(scheduled).await {
                            error!("prove failed: {e:#}");
                        }
                    });
                }

                while tasks.join_next().await.is_some() {}
                error!("prove stage shutting down");
            }
        });

        while let Some(result) = subscription.next().await {
            match result {
                Ok(event) => {
                    let blobs = event.blobs.unwrap_or_default();
                    event_tx
                        .send(BlockEvent::new(event.height, blobs))
                        .await
                        .map_err(|_| anyhow::anyhow!("worker queue closed"))?;
                }
                Err(e) => {
                    error!("Subscription error: {e}");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Retrieves the proof inputs required via RPC calls to the configured celestia and evm nodes.
    async fn prepare_inputs(self: Arc<Self>, client: Arc<Client>, event: BlockEvent) -> Result<ProofJob> {
        let extended_header = client.header_get_by_height(event.height).await?;
        let namespace_data = client
            .share_get_namespace_data(&extended_header, self.app.namespace)
            .await?;

        let proofs: Vec<NamespaceProof> = namespace_data.rows.iter().map(|row| row.proof.clone()).collect();

        let signed_data: Vec<SignedData> = event
            .blobs
            .iter()
            .filter_map(|blob| SignedData::decode(Bytes::from(blob.data.clone())).ok())
            .collect();

        let mut executor_inputs = Vec::with_capacity(signed_data.len());
        for data in signed_data {
            let block_number = data
                .data
                .as_ref()
                .and_then(|d| d.metadata.as_ref())
                .map(|m| m.height)
                .ok_or_else(|| anyhow!("missing height for SignedData"))?;

            executor_inputs.push(self.eth_client_executor_input(block_number).await?);
        }

        debug!("Got {} evm inputs at height {}", executor_inputs.len(), event.height);

        Ok(ProofJob {
            height: event.height,
            extended_header,
            proofs,
            blobs: event.blobs,
            executor_inputs,
        })
    }

    async fn prove_and_store(self: Arc<Self>, scheduled: ScheduledProofJob) -> Result<()> {
        let extended_header = &scheduled.job.extended_header;

        let inputs = BlockExecInput {
            header_raw: serde_cbor::to_vec(&extended_header.header)?,
            dah: extended_header.dah.clone(),
            blobs_raw: serde_cbor::to_vec(&scheduled.job.blobs)?,
            pub_key: self.app.pub_key.clone(),
            namespace: self.app.namespace,
            proofs: scheduled.job.proofs.clone(),
            executor_inputs: scheduled.job.executor_inputs.clone(),
            trusted_height: scheduled.trusted_height,
            trusted_root: scheduled.trusted_root,
        };

        let (proof, outputs) = self.prove(inputs).await?;

        if let Err(e) = self
            .storage
            .store_block_proof(scheduled.job.height, &proof, &outputs)
            .await
        {
            error!(
                "Failed to store proof for block {}: {} - error: {e:#}",
                scheduled.job.height, outputs,
            );
            // Note: We continue execution even if storage fails to avoid breaking the proving pipeline
        }

        info!(
            "Successfully created and stored proof for block {}. Outputs: {}",
            scheduled.job.height, outputs,
        );

        self.tx.send(ProofCommitted(scheduled.job.height)).await?;

        Ok(())
    }
}
