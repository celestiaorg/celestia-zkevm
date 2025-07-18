#![allow(dead_code)]
use std::fs;
use std::result::Result::{Err, Ok};
use std::str::FromStr;
use std::sync::Arc;

use alloy_provider::ProviderBuilder;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use celestia_rpc::{client::Client, BlobClient, HeaderClient, ShareClient};
use celestia_types::{nmt::Namespace, Blob, ExtendedHeader, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use nmt_rs::{
    simple_merkle::{db::MemDb, proof::Proof, tree::MerkleTree},
    TmSha2Hasher,
};
use prost::Message;
use reth_chainspec::ChainSpec;
use rollkit_types::v1::SignedHeader;
use rollkit_types::v1::{store_service_client::StoreServiceClient, GetMetadataRequest, SignedData};
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    include_elf, EnvProver, HashableKey, ProverClient, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin,
    SP1VerifyingKey,
};
use tendermint::block::Header;
use tendermint_proto::{
    v0_38::{types::BlockId as RawBlockId, version::Consensus as RawConsensusVersion},
    Protobuf,
};

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
        let genesis = Genesis::from_str(&raw_genesis)?;
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

    /// Queries the inclusion height for the given EVM block number from the sequencer rpc.
    async fn inclusion_height(&self, block_number: u64) -> Result<u64> {
        let mut client = StoreServiceClient::connect(self.app.sequencer_rpc.clone()).await?;
        let req = GetMetadataRequest {
            key: format!("rhb/{}/d", block_number),
        };

        let resp = client.get_metadata(req).await?;
        let height = u64::from_le_bytes(resp.into_inner().value[..8].try_into()?);

        Ok(height)
    }

    /// Queries the extended header from the Celestia data availability rpc.
    async fn extended_header(&self, height: u64) -> Result<ExtendedHeader> {
        let client = Client::new(&self.app.celestia_rpc, None).await?;
        let header = client.header_get_by_height(height).await?;

        Ok(header)
    }

    /// Queries all blobs from Celestia for the given inclusion height and filters them to find the blob for the
    /// corresponding EVM block number.
    async fn blob_for_height(&self, block_number: u64, inclusion_height: u64) -> Result<Blob> {
        let client = Client::new(&self.app.celestia_rpc, None).await?;

        let blobs = client
            .blob_get_all(inclusion_height, &[self.app.namespace])
            .await?
            .ok_or_else(|| anyhow::anyhow!("No blobs found at inclusion height {}", inclusion_height))?;

        println!("num blobs: {}", blobs.len());

        for blob in blobs {
            let signed_data = match SignedData::decode(blob.data.as_slice()) {
                Ok(data) => data,
                Err(e) => {
                    println!("Failed to decode SignedData: {:?}", e);

                    let header = match SignedHeader::decode(blob.data.as_slice()) {
                        Ok(header) => header,
                        Err(e) => {
                            println!("Failed to decode SignedHeader: {:?}", e);
                            continue;
                        }
                    };

                    println!("signed header height: {:?}", header.header.unwrap().height);
                    continue;
                }
            };

            let Some(metadata) = signed_data.data.and_then(|d| d.metadata) else {
                continue;
            };

            println!("metadata.height {}", metadata.height);
            if metadata.height == block_number {
                return Ok(blob);
            }
        }

        bail!(
            "No blob found for block number {} at inclusion height {}",
            block_number,
            inclusion_height
        );
    }

    /// Queries the Celestia data availability rpc for a ShareProof for the provided blob.
    async fn blob_inclusion_proof(&self, blob: Blob, header: &ExtendedHeader) -> Result<ShareProof> {
        let client = Client::new(&self.app.celestia_rpc, None).await?;

        let eds_size = header.dah.row_roots().len() as u64;
        let ods_size = eds_size / 2;
        let first_row_index = blob.index.unwrap() / eds_size;
        let ods_index = blob.index.unwrap() - (first_row_index * ods_size);

        // NOTE: mutated the app version in the header as otherwise we run into v4 unsupported issues
        let mut modified_header = header.clone();
        modified_header.header.version.app = 3;

        let range_response = client
            .share_get_range(&modified_header, ods_index, ods_index + blob.shares_len() as u64)
            .await?;

        let share_proof = range_response.proof;
        share_proof.verify(modified_header.dah.hash())?;

        Ok(share_proof)
    }

    /// Creates an inclusion proof for the data availability root within the provided Header.
    fn data_root_proof(&self, header: &ExtendedHeader) -> Result<(Vec<u8>, Proof<TmSha2Hasher>)> {
        let mut tree = MerkleTree::<MemDb<[u8; 32]>, TmSha2Hasher>::with_hasher(TmSha2Hasher::new());

        for field in self.prepare_header_fields(header) {
            tree.push_raw_leaf(&field);
        }

        let data_hash_index = 6;
        let (data_hash, proof) = tree.get_index_with_proof(data_hash_index);

        assert_eq!(header.hash().as_ref(), tree.root());

        Ok((data_hash, proof))
    }

    /// Prepares leaf data for the provided Header.
    fn prepare_header_fields(&self, header: &ExtendedHeader) -> Vec<Vec<u8>> {
        let h = &header.header;

        vec![
            Protobuf::<RawConsensusVersion>::encode_vec(h.version),
            h.chain_id.clone().encode_vec(),
            h.height.encode_vec(),
            h.time.encode_vec(),
            Protobuf::<RawBlockId>::encode_vec(h.last_block_id.unwrap_or_default()),
            h.last_commit_hash.unwrap_or_default().encode_vec(),
            h.data_hash.unwrap_or_default().encode_vec(),
            h.validators_hash.encode_vec(),
            h.next_validators_hash.encode_vec(),
            h.consensus_hash.encode_vec(),
            h.app_hash.clone().encode_vec(),
            h.last_results_hash.unwrap_or_default().encode_vec(),
            h.evidence_hash.unwrap_or_default().encode_vec(),
            h.proposer_address.encode_vec(),
        ]
    }
}

/// Input to the EVM block execution proving circuit.
///
/// This input contains all necessary data to verify block execution and data
/// availability of blob data in Celestia.
#[derive(Serialize, Deserialize)]
pub struct BlockExecInput {
    // blob_proof is an inclusion proof of blob data availability in Celestia.
    pub blob_proof: KeccakInclusionToDataRootProofInput,
    // data_root_proof is an inclusion proof of the data root within the Celestia header.
    pub data_root_proof: Proof<TmSha2Hasher>,
    // header is the Celestia block header at which the blob data is available.
    pub header: Header,
    // state_transition_fn is the application of the blob data applied to the EVM state machine.
    pub state_transition_fn: Vec<u8>,
}

/// Output of the EVM block execution proving circuit.
///
/// This contains the resulting commitments after applying the state transition function and verifying
/// data availability in Celestia.
#[derive(Serialize, Deserialize)]
pub struct BlockExecOutput {
    // blob_commitment is the blob commitment for the EVM block.
    pub blob_commitment: [u8; 32],
    // header_hash is the hash of the EVM block header.
    pub header_hash: [u8; 32],
    // prev_header_hash is the hash of the previous EVM block header.
    pub prev_header_hash: [u8; 32],
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
        stdin.write(&input.blob_proof);
        stdin.write(&input.state_transition_fn);
        stdin.write_vec(serde_cbor::to_vec(&input.header)?);
        stdin.write(&input.data_root_proof);

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
