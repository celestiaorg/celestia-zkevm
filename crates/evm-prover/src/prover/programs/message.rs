//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use std::sync::{Arc, RwLock};

use alloy_primitives::FixedBytes;
use anyhow::Result;
use evm_hyperlane_types_sp1::{HyperlaneMessageInputs, HyperlaneMessageOutputs};
use sp1_sdk::{include_elf, EnvProver, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};

use crate::prover::{ProgramProver, ProverConfig};

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
    message_index: u64,
}

impl TrustedState {
    pub fn new(snapshot_index: u64, message_index: u64) -> Self {
        Self {
            snapshot_index,
            message_index,
        }
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

    pub fn run(self: Arc<Self>) -> Result<()> {
        Ok(())
    }
}
