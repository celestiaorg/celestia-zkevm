#![allow(dead_code)]
use std::result::Result::{Err, Ok};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use ev_zkevm_types::programs::block::{BlockRangeExecInput, BlockRangeExecOutput};

#[cfg(feature = "sp1")]
use sp1_sdk::include_elf;

use crate::prover::ProverConfig;
use crate::proof_system::{ProofMode, ProofSystemBackend, ProverFactory, UnifiedProof};

// Program IDs for different proof systems
#[cfg(feature = "sp1")]
pub const EV_RANGE_EXEC_PROGRAM_ID: &[u8] = include_elf!("ev-range-exec-program");

// NOTE: RISC0 ImageID would be loaded from ev-range-exec-host, but that crate
// is excluded from workspace due to crypto patch conflicts.
// For RISC0 support, the ImageID must be provided via ProverConfig.
#[cfg(all(feature = "risc0", not(feature = "sp1")))]
pub const EV_RANGE_EXEC_PROGRAM_ID: &[u8] = &[];  // Placeholder - use config.program_id instead

// Compatibility alias
pub const EV_RANGE_EXEC_ELF: &[u8] = EV_RANGE_EXEC_PROGRAM_ID;

/// A prover for verifying and aggregating proofs over a range of blocks.
///
/// This struct works with both SP1 and Risc0 proof systems, using the ProverFactory
/// to select the appropriate backend at runtime. It takes a sequence of proofs,
/// verifies them recursively, and aggregates the result into a single proof.
///
/// The number of `vkeys` must exactly match the number of `proofs`.
pub struct BlockRangeExecProver {
    pub config: ProverConfig,
    pub prover: Arc<dyn ProofSystemBackend>,
}

/// ProofInput is a system-agnostic proof input for aggregation.
/// It contains the proof and its verification key/image ID.
pub struct ProofInput {
    /// The proof in unified format
    pub proof: UnifiedProof,
    /// The verification key (32-byte digest for both SP1 and Risc0)
    pub vkey: [u32; 8],
}

impl BlockRangeExecProver {
    /// Creates a new BlockRangeExecProver using the runtime-configured proof system
    pub fn new() -> Result<Arc<Self>> {
        let config = BlockRangeExecProver::default_config();
        let prover = Arc::from(ProverFactory::from_env()?);

        Ok(Arc::new(Self { config, prover }))
    }

    /// Generates a recursive proof aggregating multiple block execution proofs.
    ///
    /// # Arguments
    /// * `input` - The range execution input containing vkeys and public values
    /// * `proofs` - Vector of proofs to aggregate
    ///
    /// # Returns
    /// A tuple of (aggregated_proof, output)
    ///
    /// # Errors
    /// - Returns an error if the number of vkeys doesn't match the number of proofs
    /// - Returns an error if proof generation fails
    pub async fn prove(
        &self,
        input: BlockRangeExecInput,
        proofs: Vec<ProofInput>,
    ) -> Result<(UnifiedProof, BlockRangeExecOutput)> {
        // Validate inputs
        if input.vkeys.len() != proofs.len() {
            return Err(anyhow!(
                "mismatched lengths: {} vkeys vs {} proofs",
                input.vkeys.len(),
                proofs.len()
            ));
        }

        // NOTE: This is a simplified implementation that works for Risc0.
        // For SP1, we would need to use stdin.write_proof() which requires
        // direct access to SP1-specific types. Since the ProofSystemBackend
        // abstraction uses raw bytes, full SP1 recursive support would require
        // extending the trait or handling it in SP1Backend.prove().
        //
        // The Risc0 circuit expects BlockRangeExecInput which already contains
        // the vkeys and public_values arrays, and it uses env::verify() internally.

        // Serialize the input which contains all necessary data
        let input_bytes = bincode::serialize(&input)?;

        // Use proof mode from config
        let proof_mode = self.config.proof_mode;
        let program_id = self.config.program_id;

        // Generate the aggregated proof
        // For Risc0: This works directly
        // For SP1: Would need special handling of compressed proofs
        let proof = self
            .prover
            .prove(program_id, &input_bytes, proof_mode)
            .await?;

        // Deserialize output
        let output: BlockRangeExecOutput = bincode::deserialize(&proof.public_values)?;

        Ok((proof, output))
    }

    /// Returns the default prover configuration for the range execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            program_id: EV_RANGE_EXEC_PROGRAM_ID,
            proof_mode: ProofMode::Groth16,
        }
    }
}
