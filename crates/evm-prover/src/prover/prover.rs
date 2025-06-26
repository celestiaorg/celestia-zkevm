use anyhow::Result;
use async_trait::async_trait;
use celestia_types::{Commitment, ExtendedHeader};
use eq_common::KeccakInclusionToDataRootProofInput;
use evm_exec_types::{BlockExecInput, BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput};
use nmt_rs::{simple_merkle::proof::Proof, TmSha2Hasher};
use sp1_sdk::{include_elf, EnvProver, SP1ProofMode, SP1ProofWithPublicValues, SP1PublicValues, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

/// Config defines metadata about the program binary (ELF), proof mode and any static keys.
pub struct Config {
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
    fn cfg(&self) -> &Config;

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

/// A prover for generating SP1 proofs for EVM block execution and data availability in Celestia.
///
/// This struct is responsible for preparing the standard input (`SP1Stdin`)
/// for a zkVM program that takes a blob inclusion proof, data root proof, Celestia Header and
/// EVM state transition function.
pub struct BlockExecProver {
    config: Config,
    prover: EnvProver,
    // extend with custom state, e.g. celestia rpc, evm rpc, etc...
}

#[async_trait]
impl ProgramProver for BlockExecProver {
    type Input = BlockExecInput;
    type Output = BlockExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &Config {
        &self.config
    }

    /// Constructs the `SP1Stdin` input required for proving.
    ///
    /// This function serializes and writes structured input data into the
    /// stdin buffer in the expected format for the SP1 program.
    ///
    /// # Errors
    /// Returns an error if serialization of any input component fails.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input.blob_proof);
        stdin.write(&input.state_transition_fn);
        stdin.write_vec(serde_cbor::to_vec(&input.header)?);
        stdin.write(&input.data_root_proof);

        Ok(stdin)
    }

    /// Parses the `SP1PublicValues` from the proof and converts it into the
    /// program's custom output type.
    ///
    /// # Errors
    /// - Returns an error if deserialization fails.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<BlockExecOutput>(proof.public_values.as_slice())?)
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

impl BlockExecProver {
    async fn generate_stf(&self, _block_number: u64) -> Result<Vec<u8>> {
        unimplemented!("TODO: RSP generation of state transition func (client_executor_input)")
    }

    async fn inclusion_height(&self, _block_number: u64) -> Result<(u64, Commitment)> {
        unimplemented!("TODO: Query rollkit rpc for DA inclusion height");
    }

    async fn blob_inclusion_proof(
        &self,
        _inclusion_height: u64,
        _commitment: Commitment,
    ) -> Result<(KeccakInclusionToDataRootProofInput, ExtendedHeader)> {
        unimplemented!("TODO: Query celestia rpc and construct blob inclusion proof");
    }

    fn data_root_proof(&self, _header: &ExtendedHeader) -> Result<(Vec<u8>, Proof<TmSha2Hasher>)> {
        unimplemented!("TODO: Build the data root to header hash inclusion proof")
    }

    /* ...additional helpers fns */
}

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
    config: Config,
    prover: EnvProver,
}

#[async_trait]
impl ProgramProver for BlockRangeExecProver {
    type Input = BlockRangeExecInput;
    type Output = BlockRangeExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &Config {
        &self.config
    }

    /// Constructs the SP1Stdin by serializing:
    /// - Verifier key digests (`vkeys`)
    /// - Public inputs for each proof
    /// - The compressed proofs themselves
    ///
    /// # Errors
    /// - Returns an error if any proof is not in compressed format.
    /// - Returns an error if the number of `proofs` and `vkeys` do not match.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();

        let vkeys = input.vkeys;
        stdin.write(&vkeys);

        let (proofs, public_values): (Vec<_>, Vec<_>) = input
            .proofs
            .into_iter()
            .map(|p| (p.proof, p.public_values.to_vec()))
            .unzip();

        if proofs.len() != vkeys.len() {
            return Err(anyhow::anyhow!(
                "Mismatch between number of proofs ({}) and vkeys ({})",
                proofs.len(),
                vkeys.len()
            ));
        }

        stdin.write(&public_values);

        for (proof, vk_digest) in proofs.iter().zip(vkeys.iter()) {
            match proof {
                SP1Proof::Compressed(inner) => {
                    stdin.write_proof(inner.clone(), *vk_digest);
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
