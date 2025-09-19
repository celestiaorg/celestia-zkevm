#![allow(dead_code)]
use std::result::Result::{Err, Ok};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ev_exec_types::{BlockRangeExecInput, BlockRangeExecOutput};
use sp1_sdk::{
    include_elf, EnvProver, ProverClient, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};

use crate::prover::{ProgramProver, ProverConfig};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_RANGE_EXEC_ELF: &[u8] = include_elf!("ev-range-exec-program");

/// A prover for verifying and aggregating SP1 proofs over a range of blocks.
///
/// This struct is responsible for preparing the standard input (`SP1Stdin`)
/// for a zkVM program that takes a sequence of SP1 proofs, their corresponding
/// public inputs, and verifier key digests. The program then verifies them
/// reducing the result to a single groth16 proof.
///
///
/// - All SP1 proofs must be in compressed format (`SP1Proof::Compressed`).
/// - The number of `vkeys` must exactly match the number of `proofs`.
pub struct BlockRangeExecProver {
    config: ProverConfig,
    prover: EnvProver,
}

/// ProofInput is a convenience type used for proof aggregation inputs within the BlockRangeExecProver program.
pub struct ProofInput {
    proof: SP1Proof,
    vkey: SP1VerifyingKey,
}

#[async_trait]
impl ProgramProver for BlockRangeExecProver {
    type Input = (BlockRangeExecInput, Vec<ProofInput>);
    type Output = BlockRangeExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &ProverConfig {
        &self.config
    }

    /// Constructs the SP1Stdin by serializing the program inputs:
    /// - Verifier key digests (`vkeys`)
    /// - Public inputs for each proof
    /// - The compressed SP1 proofs and their associated verifying keys.
    ///
    /// # Errors
    /// - Returns an error if any proof is not in compressed format.
    /// - Returns an error if the number of `proofs` and `vkeys` do not match.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();

        let (inputs, proof_inputs) = input;
        if inputs.vkeys.len() != proof_inputs.len() {
            return Err(anyhow!(
                "mismatched lengths: {} vkeys vs {} proof_inputs",
                inputs.vkeys.len(),
                proof_inputs.len()
            ));
        }

        stdin.write(&inputs);
        for proof_input in proof_inputs.iter() {
            match &proof_input.proof {
                SP1Proof::Compressed(inner) => {
                    stdin.write_proof(*inner.clone(), proof_input.vkey.vk.clone());
                }
                _ => {
                    return Err(anyhow::anyhow!("Expected compressed SP1 proof"));
                }
            }
        }

        Ok(stdin)
    }

    /// Parses the `SP1PublicValues` from the proof and converts it into the
    /// program's custom output type.
    ///
    /// # Errors
    /// - Returns an error if deserialization fails.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<BlockRangeExecOutput>(
            proof.public_values.as_slice(),
        )?)
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

impl BlockRangeExecProver {
    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EV_RANGE_EXEC_ELF,
            proof_mode: SP1ProofMode::Groth16,
        }
    }
}

impl Default for BlockRangeExecProver {
    /// Creates a new instance of [`BlockRangeExecProver`] using default configuration
    /// and prover environment settings.
    fn default() -> Self {
        let config = BlockRangeExecProver::default_config();
        let prover = ProverClient::from_env();
        Self { config, prover }
    }
}
