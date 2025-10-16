use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use sp1_sdk::{HashableKey, SP1ProofMode, SP1ProvingKey, SP1VerifyingKey};
use tracing::warn;

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

// BaseProverConfig defines a core capability trait for configs used by a ProgramProver.
pub trait BaseProverConfig {
    fn pk(&self) -> Arc<SP1ProvingKey>;
    fn vk(&self) -> Arc<SP1VerifyingKey>;
    fn proof_mode(&self) -> SP1ProofMode;
}

/// ProverConfig defines program configuration such as proof mode and associated keys.
#[derive(Clone)]
pub struct ProverConfig {
    pub pk: Arc<SP1ProvingKey>,
    pub vk: Arc<SP1VerifyingKey>,
    pub proof_mode: SP1ProofMode,
}

impl ProverConfig {
    pub fn new(pk: SP1ProvingKey, vk: SP1VerifyingKey, mode: SP1ProofMode) -> Self {
        ProverConfig {
            pk: Arc::new(pk),
            vk: Arc::new(vk),
            proof_mode: mode,
        }
    }
}

impl BaseProverConfig for ProverConfig {
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

/// RecursiveProverConfig defines program configuration such as proof mode and associated keys as well as
/// a map containing verifying keys for recursive proof verification.
#[derive(Clone)]
pub struct RecursiveProverConfig {
    pub pk: Arc<SP1ProvingKey>,
    pub vk: Arc<SP1VerifyingKey>,
    pub proof_mode: SP1ProofMode,
    pub inner: HashMap<ProgramId, ProgramVerifyingKey>,
}

impl RecursiveProverConfig {
    pub fn new(
        pk: SP1ProvingKey,
        vk: SP1VerifyingKey,
        mode: SP1ProofMode,
        inner: HashMap<ProgramId, ProgramVerifyingKey>,
    ) -> Self {
        RecursiveProverConfig {
            pk: Arc::new(pk),
            vk: Arc::new(vk),
            proof_mode: mode,
            inner,
        }
    }
}

impl BaseProverConfig for RecursiveProverConfig {
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

pub type ProgramId = &'static str; // TODO: maybe enum...

#[derive(Clone)]
pub struct ProgramVerifyingKey {
    pub vk: Arc<SP1VerifyingKey>,
    pub digest: [u32; 8],
}

impl ProgramVerifyingKey {
    pub fn new(vk: Arc<SP1VerifyingKey>) -> Self {
        let digest = vk.vk.hash_u32();
        Self { vk, digest }
    }
}
