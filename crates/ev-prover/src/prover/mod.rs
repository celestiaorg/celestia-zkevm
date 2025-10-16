use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Display, result::Result::Ok};

use anyhow::Result;
use async_trait::async_trait;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{network::NetworkMode, Prover, ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use tracing::debug;

#[allow(clippy::module_inception)]
pub mod config;
pub mod programs;
pub mod service;

pub use config::{BaseProverConfig, ProgramId, ProgramVerifyingKey, ProverConfig, ProverMode, RecursiveProverConfig};

pub type SP1Prover = dyn Prover<CpuProverComponents> + Send + Sync;

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

        let proof = self.prover().prove(&cfg.pk(), &stdin, cfg.proof_mode())?;

        let output = self.post_process(proof.clone())?;
        Ok((proof, output))
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> Arc<SP1Prover>;

    /// Parse or convert program outputs.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output>;
}

/// Construct a prover based on the SP1_PROVER environment variable.
pub fn prover_from_env() -> Result<Arc<SP1Prover>> {
    let mode_str = env::var("SP1_PROVER").unwrap_or_else(|_| "MOCK".to_string());
    let mode: ProverMode = ProverMode::from_str(&mode_str).unwrap();

    let prover: Arc<SP1Prover> = match mode {
        ProverMode::Mock => {
            debug!("Using mock prover backend");
            Arc::new(ProverClient::builder().mock().build())
        }
        ProverMode::Cpu => {
            debug!("Using CPU prover backend");
            Arc::new(ProverClient::builder().cpu().build())
        }
        ProverMode::Cuda => {
            debug!("Using CUDA prover backend");
            Arc::new(ProverClient::builder().cuda().build())
        }
        ProverMode::Network => {
            debug!("Using network prover backend");
            Arc::new(
                ProverClient::builder()
                    .network_for(NetworkMode::Mainnet)
                    .rpc_url("https://rpc.mainnet.succinct.xyz")
                    .build(),
            )
        }
    };

    Ok(prover)
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
