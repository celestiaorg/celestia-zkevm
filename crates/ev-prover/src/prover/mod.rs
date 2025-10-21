use std::result::Result::Ok;

use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "sp1")]
use sp1_sdk::{EnvProver, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};

use crate::proof_system::ProofMode;

#[allow(clippy::module_inception)]
pub mod programs;
pub mod service;

/// ProverConfig defines metadata about the program binary and proof mode.
/// The program_id is a generic identifier that works for both SP1 (ELF) and RISC0 (ImageID).
pub struct ProverConfig {
    pub program_id: &'static [u8],
    pub proof_mode: ProofMode,
}

/// ProgramProver is a legacy trait implemented per SP1 program.
/// NOTE: This trait is deprecated and only kept for backward compatibility.
/// New code should use ProofSystemBackend directly.
///
/// Associated types let each program pick its own Input and Output context.
#[cfg(feature = "sp1")]
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

        let (pk, _vk) = self.prover().setup(cfg.program_id);

        // Convert ProofMode to SP1ProofMode
        let sp1_mode = match cfg.proof_mode {
            ProofMode::Core => SP1ProofMode::Core,
            ProofMode::Compressed => SP1ProofMode::Compressed,
            ProofMode::Groth16 => SP1ProofMode::Groth16,
            ProofMode::Plonk => SP1ProofMode::Plonk,
        };

        let proof: SP1ProofWithPublicValues = match sp1_mode {
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
