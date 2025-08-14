#![allow(dead_code)]
use std::fs;
use std::result::Result::{Err, Ok};
use std::sync::Arc;

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_provider::ProviderBuilder;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use celestia_rpc::{client::Client, BlobClient, HeaderClient, ShareClient};
use celestia_types::nmt::{Namespace, NamespaceProof};
use celestia_types::{Blob, DataAvailabilityHeader, ExtendedHeader};
use ev_types::v1::SignedData;
use prost::Message;
use reth_chainspec::ChainSpec;
use rsp_client_executor::io::EthClientExecutorInput;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    include_elf, EnvProver, HashableKey, ProverClient, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin,
    SP1VerifyingKey,
};
use tendermint::block::Header;

use crate::config::config::{Config, APP_HOME, CONFIG_DIR, GENESIS_FILE};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

/// ProverConfig defines metadata about the program binary (ELF), proof mode and any static keys.
pub struct ProverConfig {
    pub elf: &'static [u8],
    pub proof_mode: SP1ProofMode,
}

/// ProgramProver is a trait implemented per SP1 program*.
///
/// Associated types let each program pick its own Input and Output context.
#[async_trait]
pub trait ProgramProver {
    /// Context needed to build the stdin for this program.
    type Input: Send + 'static;
    /// Output data to return alongside the proof.
    type Output: Send + 'static;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &ProverConfig;

    /// Build the program stdin from the prover inputs.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin>;

    /// Prove produces a proof and parsed outputs.
    /// The default implementation matches the configured proof mode and program elf from the prover config.
    async fn prove(&self, input: Self::Input) -> Result<(SP1ProofWithPublicValues, Self::Output)> {
        let cfg = self.cfg();
        let stdin = self.build_stdin(input)?;

        let (pk, _vk) = self.prover().setup(cfg.elf);

        let proof: SP1ProofWithPublicValues = match cfg.proof_mode {
            SP1ProofMode::Core => self.prover().prove(&pk, &stdin).core().run()?,
            SP1ProofMode::Compressed => self.prover().prove(&pk, &stdin).compressed().run()?,
            SP1ProofMode::Groth16 => self.prover().prove(&pk, &stdin).groth16().run()?,
            SP1ProofMode::Plonk => self.prover().prove(&pk, &stdin).plonk().run()?,
        };

        let output = self.post_process(proof.clone())?;

        Ok((proof, output))
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver;

    /// Parse or convert program outputs.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output>;
}

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
    pub sequencer_rpc: String,
}

impl AppContext {
    pub fn from_config(config: Config) -> Result<Self> {
        let genesis = AppContext::load_genesis().context("Error loading app genesis")?;

        let chain_spec = Arc::new(
            (&genesis)
                .try_into()
                .map_err(|e| anyhow!("Failed to convert genesis to chain spec: {e}"))?,
        );

        let raw_ns = hex::decode(config.namespace_hex)?;
        let namespace = Namespace::new_v0(raw_ns.as_ref()).context("Failed to construct Namespace")?;

        Ok(AppContext {
            chain_spec,
            genesis,
            namespace,
            celestia_rpc: config.celestia_rpc,
            evm_rpc: config.evm_rpc,
            sequencer_rpc: config.sequencer_rpc,
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
}

impl BlockExecProver {
    /// Creates a new instance of [`BlockExecProver`] for the provided [`AppContext`] using default configuration
    /// and prover environment settings.
    pub fn new(app: AppContext) -> Self {
        let config = BlockExecProver::default_config();
        let prover = ProverClient::from_env();

        Self { app, config, prover }
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_EXEC_ELF,
            proof_mode: SP1ProofMode::Compressed,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let client = Client::new(&self.app.celestia_rpc, None).await?;
        let mut subscription = client.blob_subscribe(self.app.namespace).await?;

        while let Some(result) = subscription.next().await {
            match result {
                Ok(event) => {
                    let blobs = event.blobs.unwrap_or_default();

                    println!(
                        "Received blob event for height: {}, blobs: {}",
                        event.height,
                        blobs.len()
                    );

                    let mut extended_header = client.header_get_by_height(event.height).await?;
                    extended_header.header.version.app = 3; // TODO: need rs support for newer celestia app versions

                    let namespace_data = client
                        .share_get_namespace_data(&extended_header, self.app.namespace)
                        .await?;

                    let _proofs: Vec<NamespaceProof> =
                        namespace_data.rows.iter().map(|row| row.proof.clone()).collect();

                    // Determine EthClientExecutorInputs for celestia block
                    let _signed_data: Vec<SignedData> = blobs
                        .into_iter()
                        .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
                        .collect();
                }
                Err(e) => {
                    eprintln!("Subscription error: {e}");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Generates the serialized state transition function (STF) input for a given EVM block number.
    async fn generate_stf(&self, block_number: u64) -> Result<Vec<u8>> {
        let host_executor = EthHostExecutor::eth(self.app.chain_spec.clone(), None);
        let provider = ProviderBuilder::new().connect_http(self.app.evm_rpc.parse()?);
        let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

        let client_input = host_executor
            .execute(block_number, &rpc_db, &provider, self.app.genesis.clone(), None, false)
            .await?;

        Ok(bincode::serialize(&client_input)?)
    }
}

/// Input to the EVM block execution proving circuit.
///
/// This input contains all necessary data to verify block execution and data
/// availability of blob data in Celestia.
#[derive(Serialize, Deserialize)]
pub struct BlockExecInput {
    // header is the Celestia block header at which the blob data is available.
    pub header: Header,
    // dah is the Celestia data availability header.
    pub dah: DataAvailabilityHeader,
    // blobs is the collection of blobs included in the namespace for the current block.
    pub blobs: Vec<Blob>,
    // pub_key is the ed25519 public key of the sequencer.
    pub pub_key: Vec<u8>,
    // namespace is the Celestia namespace to which EVM block data blobs are submitted.
    pub namespace: Namespace,
    // proofs is a collection of Namespaced Merkle Tree proofs for the included blob data.
    pub proofs: Vec<NamespaceProof>,
    // executor_inputs is the collection of state transition functions for each EVM block included in the Celestia block.
    pub executor_inputs: Vec<EthClientExecutorInput>,
}

/// Output of the EVM block execution proving circuit.
///
/// This contains the resulting commitments after applying the state transition function and verifying
/// data availability in Celestia.
#[derive(Serialize, Deserialize)]
pub struct BlockExecOutput {
    // celestia_header_hash is the merkle hash of the Celestia block header.
    pub celestia_header_hash: [u8; 32],
    // prev_celestia_header_hash is the merkle hash of the previous Celestia block header.
    pub prev_celestia_header_hash: [u8; 32],
    // new_height is the block number after the state transition function has been applied.
    pub new_height: u64,
    // new_state_root is the EVM application state root after the state transition function has been applied.
    pub new_state_root: [u8; 32],
    // prev_height is the block number before the state transition function has been applied.
    pub prev_height: u64,
    // prev_state_root is the EVM application state root before the state transition function has been applied.
    pub prev_state_root: [u8; 32],
    // namespace is the Celestia namespace that contains the blob data.
    pub namespace: Namespace,
    // public_key is the sequencer's public key used to verify the signatures of the signed data.
    pub public_key: [u8; 32],
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
        stdin.write_vec(serde_cbor::to_vec(&input.header)?);

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

/// Input to a batch execution proving system that verifies multiple SP1 proofs
/// for a range of EVM blocks.
///
/// This is used to verify that N state transitions have occurred using a single
/// verifying key, producing a new application state root.
#[derive(Serialize, Deserialize)]
pub struct BlockRangeExecInput {
    // proofs is a vector of SP1 proofs with their associated public values
    pub proofs: Vec<SP1ProofWithPublicValues>,
    // vkey is the SP1 verifier key for verifying proofs
    pub vkey: SP1VerifyingKey,
}

/// Output of a batch execution proof that validates N state transitions
/// from a trusted starting point.
///
/// This contains the resulting commitments after applying a sequence of verified SP1 proofs,
/// advancing the EVM application state from `trusted_height` to `new_height`.
#[derive(Serialize, Deserialize)]
pub struct BlockRangeExecOutput {
    // celestia_header_hash is the hash of the celestia header at which new_height is available.
    pub celestia_header_hash: [u8; 32],
    // trusted_height is the trusted height of the EVM application.
    pub trusted_height: u64,
    // trusted_state_root is the state commitment root of the EVM application at trusted_height.
    pub trusted_state_root: [u8; 32],
    // new_height is the EVM application block number after N state transitions.
    pub new_height: u64,
    // new_state_root is the computed state root of the EVM application after
    // executing N blocks from trusted_height to new_height.
    pub new_state_root: [u8; 32],
}

#[async_trait]
impl ProgramProver for BlockRangeExecProver {
    type Input = BlockRangeExecInput;
    type Output = BlockRangeExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &ProverConfig {
        &self.config
    }

    /// Constructs the SP1Stdin by serializing:
    /// - Verifier key digests (`vkeys`)
    /// - Public inputs for each proof
    /// - The compressed proofs themselves
    ///
    /// # Errors
    /// - Returns an error if any proof is not in compressed format.
    /// - Returns an error if the number of `proofs` and `vkeys` do not match.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();

        let (proofs, public_values): (Vec<_>, Vec<_>) = input
            .proofs
            .into_iter()
            .map(|p| (p.proof, p.public_values.to_vec()))
            .unzip();

        let vkeys = vec![input.vkey.hash_u32(); proofs.len()];
        stdin.write(&vkeys);
        stdin.write(&public_values);

        for proof in proofs.iter() {
            match proof {
                SP1Proof::Compressed(inner) => {
                    stdin.write_proof(*inner.clone(), input.vkey.vk.clone());
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
