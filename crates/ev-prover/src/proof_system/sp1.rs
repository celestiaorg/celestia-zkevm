/// SP1 proof system backend implementation

use super::{ProofMode, ProofSystemBackend, ProofSystemType, UnifiedProof};
use anyhow::Result;
use async_trait::async_trait;
use sp1_sdk::{EnvProver, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};

/// SP1Backend wraps the SP1 prover client
pub struct SP1Backend {
    client: EnvProver,
}

impl SP1Backend {
    /// Create a new SP1 backend from environment
    ///
    /// # Mock Mode
    /// Set `SP1_PROVER=mock` environment variable to enable mock mode (fast execution without proofs).
    /// Mock mode is useful for testing but provides no security guarantees.
    pub fn new() -> Result<Self> {
        let client = ProverClient::from_env();

        // Check if mock mode is enabled
        if let Ok(prover_mode) = std::env::var("SP1_PROVER") {
            if prover_mode == "mock" {
                tracing::warn!("⚠️  SP1_PROVER=mock is enabled - proofs will NOT be generated!");
                tracing::warn!("⚠️  This mode is for development only and provides NO security!");
            }
        }

        Ok(Self { client })
    }

    /// Convert our generic ProofMode to SP1's proof mode
    fn to_sp1_mode(mode: ProofMode) -> SP1ProofMode {
        match mode {
            ProofMode::Core => SP1ProofMode::Core,
            ProofMode::Compressed => SP1ProofMode::Compressed,
            ProofMode::Groth16 => SP1ProofMode::Groth16,
            ProofMode::Plonk => SP1ProofMode::Plonk,
        }
    }

    /// Convert SP1ProofWithPublicValues to UnifiedProof
    fn to_unified_proof(proof: SP1ProofWithPublicValues) -> Result<UnifiedProof> {
        let proof_bytes = bincode::serialize(&proof.proof)?;
        let public_values = proof.public_values.to_vec();
        Ok(UnifiedProof::new(
            ProofSystemType::SP1,
            proof_bytes,
            public_values,
        ))
    }

    /// Convert UnifiedProof back to SP1ProofWithPublicValues
    fn from_unified_proof(proof: &UnifiedProof) -> Result<SP1ProofWithPublicValues> {
        if proof.system != ProofSystemType::SP1 {
            anyhow::bail!("Proof is not from SP1 system");
        }
        bincode::deserialize(&proof.proof_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize SP1 proof: {}", e))
    }
}

#[async_trait]
impl ProofSystemBackend for SP1Backend {
    fn system_type(&self) -> ProofSystemType {
        ProofSystemType::SP1
    }

    async fn prove(
        &self,
        program_id: &[u8],
        input: &[u8],
        proof_mode: ProofMode,
    ) -> Result<UnifiedProof> {
        // Deserialize input as SP1Stdin
        let stdin: SP1Stdin = bincode::deserialize(input)?;

        // Setup proving and verifying keys
        let (pk, _vk) = self.client.setup(program_id);

        // Generate proof based on mode
        let sp1_mode = Self::to_sp1_mode(proof_mode);
        let proof: SP1ProofWithPublicValues = match sp1_mode {
            SP1ProofMode::Core => self.client.prove(&pk, &stdin).core().run()?,
            SP1ProofMode::Compressed => self.client.prove(&pk, &stdin).compressed().run()?,
            SP1ProofMode::Groth16 => self.client.prove(&pk, &stdin).groth16().run()?,
            SP1ProofMode::Plonk => self.client.prove(&pk, &stdin).plonk().run()?,
        };

        Self::to_unified_proof(proof)
    }

    fn verify(&self, program_id: &[u8], proof: &UnifiedProof) -> Result<bool> {
        let sp1_proof = Self::from_unified_proof(proof)?;
        let (_pk, vk) = self.client.setup(program_id);
        self.client
            .verify(&sp1_proof, &vk)
            .map(|_| true)
            .or(Ok(false))
    }

    fn public_values(&self, proof: &UnifiedProof) -> Result<Vec<u8>> {
        Ok(proof.public_values.clone())
    }
}

impl Default for SP1Backend {
    fn default() -> Self {
        Self::new().expect("Failed to create SP1 backend")
    }
}
