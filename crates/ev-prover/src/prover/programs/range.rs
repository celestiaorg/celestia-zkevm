#![allow(dead_code)]
use std::{collections::BTreeSet, sync::Arc};

use anyhow::{anyhow, Ok, Result};
use async_trait::async_trait;
use ev_zkevm_types::programs::block::{BlockRangeExecInput, BlockRangeExecOutput};
use sp1_sdk::{
    include_elf, EnvProver, ProverClient, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use storage::proofs::ProofStorage;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, info};

use crate::prover::{ProgramProver, ProofCommitted, ProverConfig};

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
    pending: BTreeSet<ProofCommitted>,
    rx: Receiver<ProofCommitted>,
    storage: Arc<dyn ProofStorage>,
    next_expected: u64, // TODO: consider persisting this; initialize to last_aggregated_end + 1
    batch_size: u64,    // e.g. 10, should be configurable
}

/// ProofInput is a convenience type used for proof aggregation inputs within the BlockRangeExecProver program.
pub struct ProofInput {
    proof: SP1Proof,
    vkey: SP1VerifyingKey,
}

impl ProofInput {
    pub fn new(proof: SP1Proof, vkey: SP1VerifyingKey) -> Self {
        Self { proof, vkey }
    }
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
    pub fn new(rx: Receiver<ProofCommitted>, storage: Arc<dyn ProofStorage>) -> Result<Self> {
        let config = BlockRangeExecProver::default_config();
        let prover = ProverClient::from_env();
        let pending = BTreeSet::new();

        Ok(Self {
            config,
            prover,
            pending,
            rx,
            storage,
            batch_size: 10,
            next_expected: 1, // TODO: initialise
        })
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EV_RANGE_EXEC_ELF,
            proof_mode: SP1ProofMode::Groth16,
        }
    }

    /// Starts the range prover loop.
    pub async fn run(mut self) -> Result<()> {
        while let Some(event) = self.rx.recv().await {
            info!("ProofCommitted for height: {}", event);
            self.pending.insert(event);

            debug!("size pending: {}", self.pending.len());

            // Drain as many back-to-back batches as are ready now.
            while let Some((start, end_inclusive)) = self.next_provable_range()? {
                self.aggregate_range(start, end_inclusive).await?;
            }
        }

        Ok(())
    }

    /// Calculate the next provable range bounded by batch size.
    /// If a complete batch exists then remove those entries from `pending`, advance the cursor, and return the range.
    /// Note: the start and end range indices are inclusive.
    fn next_provable_range(&mut self) -> Result<Option<(u64, u64)>> {
        let start = self.next_expected;
        let end = start + self.batch_size - 1;

        let Some(min) = self.pending.first() else {
            return Ok(None); // empty set
        };

        if min.height() > start {
            return Ok(None); // missing start element
        }

        // Walk the ordered set from `start` and ensure we have exactly `batch_size` elements.
        let mut cursor = start;
        let iter = self.pending.range(ProofCommitted(start)..).peekable();
        for proof in iter.take(self.batch_size as usize) {
            if proof.height() != cursor {
                return Ok(None); // missing contiguous element, incomplete batch
            }
            cursor += 1;
        }

        // Ensure batch is complete
        if cursor <= end {
            return Ok(None);
        }

        // Complete batch, remove elements and advance to next height.
        for h in start..=end {
            self.pending.remove(&ProofCommitted(h));
        }

        self.next_expected = cursor;
        Ok(Some((start, end)))
    }

    /// Aggregates a range of block proofs, start and end inclusive.
    async fn aggregate_range(&mut self, _start: u64, _end: u64) -> Result<()> {
        Ok(())
    }
}
