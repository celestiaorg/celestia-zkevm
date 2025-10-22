/// Proof system abstraction layer for supporting multiple zkVM backends (SP1, Risc0, etc.)
///
/// This module provides a unified interface for working with different proof systems.
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "risc0")]
pub mod risc0;
#[cfg(feature = "sp1")]
pub mod sp1;

/// ProofSystemType identifies which proof system to use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSystemType {
    #[cfg(feature = "sp1")]
    SP1,
    #[cfg(feature = "risc0")]
    Risc0,
}

impl ProofSystemType {
    /// Parse from environment variable or string
    /// Returns the default proof system based on enabled features
    pub fn from_env() -> Self {
        std::env::var("PROOF_SYSTEM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Self::default)
    }
}

impl Default for ProofSystemType {
    fn default() -> Self {
        // Prefer SP1 if available, otherwise use Risc0
        #[cfg(feature = "sp1")]
        return Self::SP1;

        #[cfg(all(feature = "risc0", not(feature = "sp1")))]
        return Self::Risc0;

        // This should never happen since we require at least one feature
        #[cfg(not(any(feature = "sp1", feature = "risc0")))]
        compile_error!("At least one proof system feature (sp1 or risc0) must be enabled");
    }
}

impl std::str::FromStr for ProofSystemType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            #[cfg(feature = "sp1")]
            "sp1" => Ok(Self::SP1),
            #[cfg(feature = "risc0")]
            "risc0" => Ok(Self::Risc0),
            _ => {
                #[cfg(all(feature = "sp1", feature = "risc0"))]
                let available = "sp1, risc0";
                #[cfg(all(feature = "sp1", not(feature = "risc0")))]
                let available = "sp1";
                #[cfg(all(feature = "risc0", not(feature = "sp1")))]
                let available = "risc0";

                anyhow::bail!("Unknown proof system '{}'. Available options: {}", s, available)
            }
        }
    }
}

impl std::fmt::Display for ProofSystemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "sp1")]
            Self::SP1 => write!(f, "SP1"),
            #[cfg(feature = "risc0")]
            Self::Risc0 => write!(f, "Risc0"),
        }
    }
}

/// ProofMode defines the type of proof to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofMode {
    /// Core/basic proof (fastest, largest)
    Core,
    /// Compressed proof (balanced)
    Compressed,
    /// Groth16 SNARK (slowest to generate, smallest, fastest to verify)
    Groth16,
    /// PLONK proof
    Plonk,
}

/// UnifiedProof is a wrapper that can hold proofs from different systems
#[derive(Clone, Serialize, Deserialize)]
pub struct UnifiedProof {
    pub system: ProofSystemType,
    pub proof_bytes: Vec<u8>,
    pub public_values: Vec<u8>,
}

impl UnifiedProof {
    pub fn new(system: ProofSystemType, proof_bytes: Vec<u8>, public_values: Vec<u8>) -> Self {
        Self {
            system,
            proof_bytes,
            public_values,
        }
    }
}

/// ProofSystemBackend is the main trait that different proof systems must implement
#[async_trait]
pub trait ProofSystemBackend: Send + Sync {
    /// Get the type of this proof system
    fn system_type(&self) -> ProofSystemType;

    /// Prove a program with the given input
    async fn prove(&self, program_id: &[u8], input: &[u8], proof_mode: ProofMode) -> Result<UnifiedProof>;

    /// Verify a proof
    fn verify(&self, program_id: &[u8], proof: &UnifiedProof) -> Result<bool>;

    /// Get the public values from a proof
    fn public_values(&self, proof: &UnifiedProof) -> Result<Vec<u8>>;
}

/// ProverFactory creates the appropriate proof system backend based on configuration
pub struct ProverFactory;

impl ProverFactory {
    /// Create a proof system backend based on the type
    pub fn create(system_type: ProofSystemType) -> Result<Box<dyn ProofSystemBackend>> {
        match system_type {
            #[cfg(feature = "sp1")]
            ProofSystemType::SP1 => Ok(Box::new(sp1::SP1Backend::new()?)),
            #[cfg(feature = "risc0")]
            ProofSystemType::Risc0 => Ok(Box::new(risc0::Risc0Backend::new()?)),
        }
    }

    /// Create from environment variable
    pub fn from_env() -> Result<Box<dyn ProofSystemBackend>> {
        Self::create(ProofSystemType::from_env())
    }
}
