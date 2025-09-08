use std::result::Result::Ok;

use anyhow::Result;
use async_trait::async_trait;
use sp1_sdk::{EnvProver, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};

#[allow(clippy::module_inception)]
pub mod prover;
pub mod service;

pub mod circuits;

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
