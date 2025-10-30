use std::env;
use std::sync::Arc;

use sp1_sdk::{SP1ProofMode, SP1ProvingKey, SP1VerifyingKey};
use tracing::warn;

pub const BATCH_SIZE: u64 = 1000;
pub const MIN_BATCH_SIZE: u64 = 10;
pub const WARN_DISTANCE: u64 = 1500;

/// ProverConfig defines a core capability trait for configs used by a ProgramProver.
pub trait ProverConfig {
    fn pk(&self) -> Arc<SP1ProvingKey>;
    fn vk(&self) -> Arc<SP1VerifyingKey>;
    fn proof_mode(&self) -> SP1ProofMode;
}

/// ProverMode defines the backend used for proving: [Mock, CPU, Cuda, Network].
#[derive(Debug, Clone, Copy)]
pub enum ProverMode {
    Mock,
    Cpu,
    Cuda,
    Network,
}

impl ProverMode {
    /// Returns the ProverMode by reading the SP1_PROVER environment variable.
    /// If SP1_PROVER is not set, this method provides a fallback of Mock mode.
    pub fn from_env() -> ProverMode {
        let mode_str = env::var("SP1_PROVER").unwrap_or_default();

        match mode_str.trim().to_ascii_lowercase().as_str() {
            "mock" => Self::Mock,
            "cpu" => Self::Cpu,
            "cuda" => Self::Cuda,
            "network" => Self::Network,
            _ => {
                warn!("SP1_PROVER unset or invalid ('{mode_str}'), defaulting to mock mode");
                Self::Mock
            }
        }
    }
}
