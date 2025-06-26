use anyhow::Result;
use async_trait::async_trait;
use celestia_types::{Commitment, ExtendedHeader};
use eq_common::KeccakInclusionToDataRootProofInput;
use nmt_rs::{simple_merkle::proof::Proof, TmSha2Hasher};
use sp1_sdk::{include_elf, EnvProver, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};

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

    /// Returns the prover config
    fn cfg(&self) -> &Config;

    /// Build the program stdin from the prover inputs.
    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin>;

    /// Prove produces a proof and parsed outputs.
    /// The default implementation matches the configured proof mode and program elf from the prover config.
    async fn prove(&self, input: Self::Input) -> Result<(SP1ProofWithPublicValues, Self::Output)> {
        let cfg = self.cfg();
        let stdin = self.build_stdin(input)?;

        // TODO: cache (pk, vk) in the cfg struct?
        let (pk, _vk) = self.prover().setup(cfg.elf);

        let proof: SP1ProofWithPublicValues = match cfg.proof_mode {
            SP1ProofMode::Core => self.prover().prove(&pk, &stdin).core().run()?,
            SP1ProofMode::Compressed => self.prover().prove(&pk, &stdin).compressed().run()?,
            SP1ProofMode::Groth16 => self.prover().prove(&pk, &stdin).groth16().run()?,
            SP1ProofMode::Plonk => self.prover().prove(&pk, &stdin).plonk().run()?,
        };

        let output = self.post_process(proof.clone());

        Ok((proof, output))
    }

    /// Returns the SP1 Prover.
    fn prover(&self) -> &EnvProver;

    /// Parse or convert program outputs.
    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Self::Output;
}

pub struct BlockExecProver {
    config: Config,
    prover: EnvProver,
    // extend with custom state, e.g. celestia rpc, evm rpc, etc...
}

pub struct BlockExecInput {/* Inputs required */}

pub struct BlockExecOutput {/* Parsed outputs */}

#[async_trait]
impl ProgramProver for BlockExecProver {
    type Input = BlockExecInput;
    type Output = BlockExecOutput;

    fn cfg(&self) -> &Config {
        &self.config
    }

    fn build_stdin(&self, _input: Self::Input) -> Result<SP1Stdin> {
        let stdin = SP1Stdin::new();
        Ok(stdin)
    }

    fn post_process(&self, _proof: SP1ProofWithPublicValues) -> Self::Output {
        let output = BlockExecOutput {};
        output
    }

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
