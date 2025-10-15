use std::{fmt::Display, result::Result::Ok};

use anyhow::Result;
use async_trait::async_trait;
use sp1_sdk::{EnvProver, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};

#[allow(clippy::module_inception)]
pub mod config;
pub mod programs;
pub mod service;

pub use config::{BaseProverConfig, ProgramId, ProgramVerifyingKey, ProverConfig, RecursiveProverConfig};

/// ProgramProver is a trait implemented per SP1 program*.
///
/// Associated types let each program pick its own Input and Output context.
#[async_trait]
pub trait ProgramProver {
    /// Config implements the the BaseProverConfig trait while allowing per implementation extensions.
    type Config: BaseProverConfig + Send + Sync + 'static;
    /// Context needed to build the stdin for this program.
    type Input: Send + 'static;
    /// Output data to return alongside the proof.
    type Output: Send + 'static;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &Self::Config;

    /// Build the program stdin from the prover inputs.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin>;

    /// Prove produces a proof and parsed outputs.
    /// The default implementation matches the configured proof mode and program elf from the prover config.
    async fn prove(&self, input: Self::Input) -> Result<(SP1ProofWithPublicValues, Self::Output)> {
        let cfg = self.cfg();
        let stdin = self.build_stdin(input)?;

        let proof: SP1ProofWithPublicValues = match cfg.proof_mode() {
            SP1ProofMode::Core => self.prover().prove(&cfg.pk(), &stdin).core().run()?,
            SP1ProofMode::Compressed => self.prover().prove(&cfg.pk(), &stdin).compressed().run()?,
            SP1ProofMode::Groth16 => self.prover().prove(&cfg.pk(), &stdin).groth16().run()?,
            SP1ProofMode::Plonk => self.prover().prove(&cfg.pk(), &stdin).plonk().run()?,
        };

        let output = self.post_process(proof.clone())?;
        Ok((proof, output))
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver;

    /// Parse or convert program outputs.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output>;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofCommitted(pub u64);

impl ProofCommitted {
    pub fn height(&self) -> u64 {
        self.0
    }
}

impl Display for ProofCommitted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
