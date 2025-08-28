#![allow(dead_code)]
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
use evm_exec_types::{BlockExecInput, BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput};
use jsonrpsee_core::client::Subscription;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use sp1_sdk::{
    include_elf, EnvProver, ProverClient, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use tokio::{
    sync::{mpsc, RwLock, Semaphore},
    task::JoinSet,
};

use crate::config::config::{Config, APP_HOME, CONFIG_DIR, GENESIS_FILE};
use crate::prover::{ProgramProver, ProverConfig};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

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

// ProofJob is a basic struct to the track proof generation per block event.
struct ProofJob {
    height: u64,
    blobs: Vec<Blob>,
}

impl ProofJob {
    fn new(height: u64, blobs: Vec<Blob>) -> Self {
        Self { height, blobs }
    }
}

// TODO: Add these as fields to the BlockExecProver to make configurable?
const QUEUE_CAP: usize = 256;
const CONCURRENCY: usize = 16;

/// A prover for generating SP1 proofs for EVM block execution and data availability in Celestia.
///
/// This struct is responsible for preparing the standard input (`SP1Stdin`)
/// for a zkVM program that takes a blob inclusion proof, data root proof, Celestia Header and
/// EVM state transition function.
pub struct BlockExecProver {
    pub app: AppContext,
    pub config: ProverConfig,
    pub prover: EnvProver,
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

impl BlockExecProver {
    /// Creates a new instance of [`BlockExecProver`] for the provided [`AppContext`] using default configuration
    /// and prover environment settings.
    pub fn new(app: AppContext) -> Arc<Self> {
        let config = BlockExecProver::default_config();
        let prover = ProverClient::from_env();

        Arc::new(Self { app, config, prover })
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_EXEC_ELF,
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

    /// Runs the block prover loop, spawning a new tokio task for each BlobsAtHeight event we receive
    /// from the celestia websocket subscription.
    ///
    /// Celestia produces a new event for each block it produces. An event may or may not contain any blobs
    /// for the configured namespace at any given height.
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let (client, mut subscription) = self.connect_and_subscribe().await?;
        let (tx, mut rx) = mpsc::channel::<ProofJob>(QUEUE_CAP);
        let sem = Arc::new(Semaphore::new(CONCURRENCY));

        // Dispatcher: receive jobs and spawn worker tasks with concurrency limit
        tokio::spawn({
            let client = client.clone();
            let prover = self.clone();
            let sem = sem.clone();

            async move {
                let mut tasks = JoinSet::new();

                while let Some(job) = rx.recv().await {
                    let client = client.clone();
                    let prover = prover.clone();

                    let permit = sem.clone().acquire_owned().await.unwrap();

                    tasks.spawn(async move {
                        let _permit = permit; // hold the slot

                        println!("Processing height: {}", job.height);
                        if let Err(e) = prover.process_job(client, &job).await {
                            println!("proof job failed {e}");
                        }
                        println!("Worker task finished for height: {}", job.height);
                    });
                }

                while tasks.join_next().await.is_some() {}
                println!("dispatcher shut down");
            }
        });

        while let Some(result) = subscription.next().await {
            match result {
                Ok(event) => {
                    let blobs = event.blobs.unwrap_or_default();
                    println!("\nNew event height={}, blobs={}", event.height, blobs.len());

                    // Backpressure: await if the queue is full,
                    // using `try_send` here would allow dropping events which we do not want.
                    tx.send(ProofJob::new(event.height, blobs))
                        .await
                        .map_err(|_| anyhow::anyhow!("worker queue closed"))?;
                }
                Err(e) => {
                    println!("Subscription error: {e}");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_job(self: Arc<Self>, client: Arc<Client>, job: &ProofJob) -> Result<()> {
        let extended_header = client.header_get_by_height(job.height).await?;
        // TODO: need rs support for newer celestia app versions.
        // Here we clone and mutate the app version in the header to avoid versioning errors when working with the rs libs.
        let mut header_clone = extended_header.clone();
        header_clone.header.version.app = 3;

        let namespace_data = client
            .share_get_namespace_data(&header_clone, self.app.namespace)
            .await?;

        let proofs: Vec<NamespaceProof> = namespace_data.rows.iter().map(|row| row.proof.clone()).collect();

        let signed_data: Vec<SignedData> = job
            .blobs
            .iter()
            .filter_map(|blob| SignedData::decode(Bytes::from(blob.data.clone())).ok())
            .collect();

        let mut executor_inputs = Vec::with_capacity(signed_data.len());
        for data in signed_data {
            // NOTE: this step is IO bound and may potentially be slow dependending on EVM block size
            // consider revisiting this if this causes synchronization issues.
            let block_number = data
                .data
                .as_ref()
                .and_then(|d| d.metadata.as_ref())
                .map(|m| m.height)
                .ok_or_else(|| anyhow::anyhow!("missing height for SignedData"))?;

            executor_inputs.push(self.eth_client_executor_input(block_number).await?);
        }

        println!("Got {} evm inputs at height {}", executor_inputs.len(), job.height);

        let (trusted_height, trusted_root) = {
            let s = self.app.trusted_state.read().await;
            (s.height, s.root)
        };

        // NOTE: The following is an optimistic update to the tracked state. If we fail to produce a proof for
        // the given inputs then we should shutdown the service, as developer intervention is likely required.
        if let Some(next) = executor_inputs.last() {
            let new_height = next.current_block.number;
            let new_state_root = next.current_block.state_root;

            let mut s = self.app.trusted_state.write().await;
            s.height = new_height;
            s.root = new_state_root;
        };

        let inputs = BlockExecInput {
            header_raw: serde_cbor::to_vec(&extended_header.header)?,
            dah: extended_header.dah,
            blobs_raw: serde_cbor::to_vec(&job.blobs.clone())?,
            pub_key: self.app.pub_key.clone(),
            namespace: self.app.namespace,
            proofs,
            executor_inputs,
            trusted_height,
            trusted_root,
        };

        // TODO: store proofs for later use by aggregation circuit.
        let (_proof, outputs) = self.prove(inputs).await?;
        println!(
            "Successfully created proof for block {}. Outputs: {}",
            job.height, outputs
        );

        Ok(())
    }
}

/// A prover for verifying and aggregating SP1 proofs over a range of blocks.
///
/// This struct is responsible for preparing the standard input (`SP1Stdin`)
/// for a zkVM program that takes a sequence of SP1 proofs, their corresponding
/// public inputs, and verifier key digests. The program then verifies them
/// reducing the result to a single groth16 proof.
///
///
/// - All SP1 proofs must be in compressed format (`SP1Proof::Compressed`).
/// - The number of `vkeys` must exactly match the number of `proofs`.
pub struct BlockRangeExecProver {
    config: ProverConfig,
    prover: EnvProver,
}

/// ProofInput is a convienience type used for proof aggregation inputs within the BlockRangeExecProver program.
pub struct ProofInput {
    proof: SP1Proof,
    vkey: SP1VerifyingKey,
}

#[async_trait]
impl ProgramProver for BlockRangeExecProver {
    type Input = (BlockRangeExecInput, Vec<ProofInput>);
    type Output = BlockRangeExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &ProverConfig {
        &self.config
    }

    /// Constructs the SP1Stdin by serializing the program inputs:
    /// - Verifier key digests (`vkeys`)
    /// - Public inputs for each proof
    /// - The compressed SP1 proofs and their associated verifying keys.
    ///
    /// # Errors
    /// - Returns an error if any proof is not in compressed format.
    /// - Returns an error if the number of `proofs` and `vkeys` do not match.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();

        let (inputs, proof_inputs) = input;
        if inputs.vkeys.len() != proof_inputs.len() {
            return Err(anyhow!(
                "mismatched lengths: {} vkeys vs {} proof_inputs",
                inputs.vkeys.len(),
                proof_inputs.len()
            ));
        }

        stdin.write(&inputs);
        for proof_input in proof_inputs.iter() {
            match &proof_input.proof {
                SP1Proof::Compressed(inner) => {
                    stdin.write_proof(*inner.clone(), proof_input.vkey.vk.clone());
                }
                _ => {
                    return Err(anyhow::anyhow!("Expected compressed SP1 proof"));
                }
            }
        }

        Ok(stdin)
    }

    /// Parses the `SP1PublicValues` from the proof and converts it into the
    /// program's custom output type.
    ///
    /// # Errors
    /// - Returns an error if deserialization fails.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<BlockRangeExecOutput>(
            proof.public_values.as_slice(),
        )?)
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

impl BlockRangeExecProver {
    /// Creates a new instance of [`BlockRangeExecProver`] using default configuration
    /// and prover environment settings.
    pub fn new() -> Self {
        let config = BlockRangeExecProver::default_config();
        let prover = ProverClient::from_env();

        Self { config, prover }
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_RANGE_EXEC_ELF,
            proof_mode: SP1ProofMode::Groth16,
        }
    }
}
