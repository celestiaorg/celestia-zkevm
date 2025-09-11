//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use alloy_primitives::{hex::FromHex, FixedBytes};
use alloy_provider::{fillers::FillProvider, Provider, ProviderBuilder};
use anyhow::Result;
use evm_hyperlane_types_sp1::{HyperlaneMessageInputs, HyperlaneMessageOutputs};
use evm_storage_proofs::client::EvmClient;
use reqwest::Url;
use sp1_sdk::{include_elf, EnvProver, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use tokio::time::sleep;

use crate::prover::{ProgramProver, ProverConfig};

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

const FREQUENCY: u64 = 50; // in blocks
const TIMEOUT: u64 = 6; // in seconds

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

pub struct AppContext {
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub trusted_state: RwLock<TrustedState>,
}

pub struct TrustedState {
    // the index of the snapshot that we will load from the db, initially 0 (empty by default)
    snapshot_index: u64,
    // the index of the last message we proofed successfully, initially 0
    height: u64,
}

impl TrustedState {
    pub fn new(snapshot_index: u64, height: u64) -> Self {
        Self { snapshot_index, height }
    }
}

pub struct HyperlaneMessageProver {
    pub app: AppContext,
    pub config: ProverConfig,
    pub prover: EnvProver,
    pub message_store: HyperlaneMessageStore,
    pub snapshot_store: HyperlaneSnapshotStore,
}

impl ProgramProver for HyperlaneMessageProver {
    type Input = HyperlaneMessageInputs;
    type Output = HyperlaneMessageOutputs;

    fn cfg(&self) -> &ProverConfig {
        &self.config
    }

    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);
        Ok(stdin)
    }

    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<HyperlaneMessageOutputs>(
            proof.public_values.as_slice(),
        )?)
    }

    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

struct ScheduledProofJob {
    height_on_chain: u64,
    state_root_on_chain: FixedBytes<32>,
}

impl HyperlaneMessageProver {
    pub fn new(
        app: AppContext,
        message_store: HyperlaneMessageStore,
        snapshot_store: HyperlaneSnapshotStore,
    ) -> Result<Arc<Self>> {
        let config = HyperlaneMessageProver::default_config();
        let prover = ProverClient::from_env();

        Ok(Arc::new(Self {
            app,
            config,
            prover,
            message_store,
            snapshot_store,
        }))
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_HYPERLANE_ELF,
            proof_mode: SP1ProofMode::Compressed,
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let evm_provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str(&self.app.evm_rpc).unwrap());
        let evm_client = EvmClient::new(evm_provider.clone());

        loop {
            // todo: get the root and height from celestia instead of directly from reth
            let (state_root_on_chain, height_on_chain) =
                simulate_get_root_and_height(&evm_provider, &evm_client).await.unwrap();

            if self.app.trusted_state.read().unwrap().height + FREQUENCY > height_on_chain {
                sleep(Duration::from_secs(TIMEOUT)).await;
                continue;
            }

            // generate a new proof for all messages that occurred since the last trusted height, inserting into the last snapshot
            // then save new snapshot
            // todo: store the proof or directly send it to celestia for verification
        }
        Ok(())
    }
}

async fn simulate_get_root_and_height(provider: &DefaultProvider, client: &EvmClient) -> Result<(FixedBytes<32>, u64)> {
    // todo: instead query celestia for a recent state root and height provided by our light client
    let height = provider.get_block_number().await.unwrap();
    let root = client.get_state_root(height).await.unwrap();
    Ok((FixedBytes::from_hex(&root).unwrap(), height))
}
