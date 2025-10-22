/// Risc0 proof system backend implementation
use super::{ProofMode, ProofSystemBackend, ProofSystemType, UnifiedProof};
use anyhow::Result;
use async_trait::async_trait;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};

/// Risc0Backend wraps the Risc0 prover client
pub struct Risc0Backend {
    // Risc0 doesn't have a persistent client like SP1's ProverClient
    // Instead, we create provers on-demand
}

impl Risc0Backend {
    /// Create a new Risc0 backend
    ///
    /// # Dev Mode
    /// Set `RISC0_DEV_MODE=1` environment variable to enable dev mode (fast execution without proofs).
    /// Dev mode is useful for testing but provides no security guarantees.
    pub fn new() -> Result<Self> {
        // Check if dev mode is enabled
        if std::env::var("RISC0_DEV_MODE").is_ok() {
            tracing::warn!("⚠️  RISC0_DEV_MODE is enabled - proofs will NOT be generated!");
            tracing::warn!("⚠️  This mode is for development only and provides NO security!");
        }
        Ok(Self {})
    }

    /// Convert our generic ProofMode to Risc0's prover options
    fn to_risc0_opts(mode: ProofMode) -> ProverOpts {
        match mode {
            // Risc0 doesn't have explicit "Core" or "Compressed" modes in the same way
            // The default prover produces STARK proofs which are similar to "compressed"
            ProofMode::Core | ProofMode::Compressed => ProverOpts::default(),

            // For Groth16, we need to use the Groth16 prover
            // This is typically done via risc0_zkvm::prove_with_groth16
            ProofMode::Groth16 => {
                // Groth16 proving is handled separately in Risc0
                ProverOpts::groth16()
            }

            // Risc0 doesn't support PLONK directly, default to STARK
            ProofMode::Plonk => ProverOpts::default(),
        }
    }

    /// Convert Risc0 Receipt to UnifiedProof
    fn to_unified_proof(receipt: Receipt) -> Result<UnifiedProof> {
        // Serialize the entire receipt (includes proof and journal/public outputs)
        let proof_bytes = bincode::serialize(&receipt)?;

        // Extract journal (public outputs) from receipt
        let public_values = receipt.journal.bytes.clone();

        Ok(UnifiedProof::new(ProofSystemType::Risc0, proof_bytes, public_values))
    }

    /// Convert UnifiedProof back to Risc0 Receipt
    fn from_unified_proof(proof: &UnifiedProof) -> Result<Receipt> {
        if proof.system != ProofSystemType::Risc0 {
            anyhow::bail!("Proof is not from Risc0 system");
        }
        bincode::deserialize(&proof.proof_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize Risc0 receipt: {}", e))
    }
}

#[async_trait]
impl ProofSystemBackend for Risc0Backend {
    fn system_type(&self) -> ProofSystemType {
        ProofSystemType::Risc0
    }

    async fn prove(&self, program_id: &[u8], input: &[u8], proof_mode: ProofMode) -> Result<UnifiedProof> {
        // Build execution environment with input data
        let env = ExecutorEnv::builder()
            .write_slice(input)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build executor environment: {}", e))?;

        // Convert program_id to ImageID
        // Risc0's ImageID is 32 bytes
        if program_id.len() != 32 {
            anyhow::bail!("Risc0 program ID must be 32 bytes, got {}", program_id.len());
        }
        let mut image_id = [0u8; 32];
        image_id.copy_from_slice(program_id);

        // Generate proof based on mode
        let receipt = match proof_mode {
            ProofMode::Groth16 => {
                // For Groth16, we need to:
                // 1. First generate a STARK proof
                // 2. Then convert it to Groth16
                let prover = default_prover();
                let opts = Self::to_risc0_opts(proof_mode);

                // Generate STARK proof first
                let prove_info = prover
                    .prove_with_opts(env, &image_id, &opts)
                    .map_err(|e| anyhow::anyhow!("Risc0 STARK proving failed: {}", e))?;

                // Convert to Groth16 (this is a placeholder - actual implementation
                // would use risc0_groth16::stark_to_snark)
                // For now, we'll return the STARK proof
                // TODO: Implement Groth16 conversion when needed
                prove_info.receipt
            }
            _ => {
                // Default STARK proving
                let prover = default_prover();
                let opts = Self::to_risc0_opts(proof_mode);

                let prove_info = prover
                    .prove_with_opts(env, &image_id, &opts)
                    .map_err(|e| anyhow::anyhow!("Risc0 proving failed: {}", e))?;

                prove_info.receipt
            }
        };

        Self::to_unified_proof(receipt)
    }

    fn verify(&self, program_id: &[u8], proof: &UnifiedProof) -> Result<bool> {
        let receipt = Self::from_unified_proof(proof)?;

        // Convert program_id to ImageID
        if program_id.len() != 32 {
            anyhow::bail!("Risc0 program ID must be 32 bytes, got {}", program_id.len());
        }
        let mut image_id = [0u8; 32];
        image_id.copy_from_slice(program_id);

        // Verify the receipt
        receipt
            .verify(image_id)
            .map(|_| true)
            .map_err(|e| anyhow::anyhow!("Risc0 verification failed: {}", e))
    }

    fn public_values(&self, proof: &UnifiedProof) -> Result<Vec<u8>> {
        Ok(proof.public_values.clone())
    }
}

impl Default for Risc0Backend {
    fn default() -> Self {
        Self::new().expect("Failed to create Risc0 backend")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risc0_backend_creation() {
        let backend = Risc0Backend::new();
        assert!(backend.is_ok());
        assert_eq!(backend.unwrap().system_type(), ProofSystemType::Risc0);
    }

    #[test]
    fn test_proof_mode_conversion() {
        // Test that proof modes convert without panicking
        let _opts_core = Risc0Backend::to_risc0_opts(ProofMode::Core);
        let _opts_compressed = Risc0Backend::to_risc0_opts(ProofMode::Compressed);
        let _opts_groth16 = Risc0Backend::to_risc0_opts(ProofMode::Groth16);
        let _opts_plonk = Risc0Backend::to_risc0_opts(ProofMode::Plonk);
    }
}
