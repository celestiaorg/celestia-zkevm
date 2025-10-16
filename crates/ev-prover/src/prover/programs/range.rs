#![allow(dead_code)]
use std::{
    collections::{BTreeSet, HashMap},
    env,
    sync::Arc,
};

use anyhow::{anyhow, Ok, Result};
use async_trait::async_trait;
use ev_zkevm_types::programs::block::{BlockRangeExecInput, BlockRangeExecOutput};
use sp1_sdk::{include_elf, SP1Proof, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey};
use storage::proofs::ProofStorage;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, info};

use crate::prover::{
    programs::block::EV_EXEC_ELF, BlockProofCommitted, ProgramProver, ProgramVerifyingKey, RangeProofCommitted,
    RecursiveProverConfig,
};
use crate::prover::{prover_from_env, SP1Prover};

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
    config: RecursiveProverConfig,
    prover: Arc<SP1Prover>,
    pending: BTreeSet<BlockProofCommitted>,
    block_rx: Receiver<BlockProofCommitted>,
    range_tx: Sender<RangeProofCommitted>,
    storage: Arc<dyn ProofStorage>,
    next_expected: Option<u64>, // TODO: consider persisting this; initialize to last_aggregated_end + 1
    batch_size: usize,
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
    type Config = RecursiveProverConfig;
    type Input = (BlockRangeExecInput, Vec<ProofInput>);
    type Output = BlockRangeExecOutput;

    /// Returns the program configuration containing the ELF and proof mode.
    fn cfg(&self) -> &Self::Config {
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
    fn prover(&self) -> Arc<SP1Prover> {
        Arc::clone(&self.prover)
    }
}

impl BlockRangeExecProver {
    pub fn new(
        batch_size: usize,
        block_rx: Receiver<BlockProofCommitted>,
        range_tx: Sender<RangeProofCommitted>,
        storage: Arc<dyn ProofStorage>,
    ) -> Result<Self> {
        let prover = prover_from_env();
        let config = BlockRangeExecProver::default_config(prover.as_ref());
        let pending = BTreeSet::new();

        Ok(Self {
            config,
            prover,
            pending,
            block_rx,
            range_tx,
            storage,
            batch_size,
            next_expected: None, // TODO: initialise from db (maybe field here isn't needed)
        })
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config(prover: &SP1Prover) -> RecursiveProverConfig {
        let (pk, vk) = prover.setup(EV_RANGE_EXEC_ELF);
        let (_, inner_vk) = prover.setup(EV_EXEC_ELF);

        RecursiveProverConfig::new(
            pk,
            vk,
            SP1ProofMode::Groth16,
            HashMap::from([("block-exec", ProgramVerifyingKey::new(Arc::new(inner_vk)))]),
        )
    }

    /// Starts the range prover loop.
    pub async fn run(mut self) -> Result<()> {
        while let Some(event) = self.block_rx.recv().await {
            info!("ProofCommitted for height: {}", event);
            self.pending.insert(event);
            debug!("Block execution proofs pending: {}", self.pending.len());

            // Drain as many batches as are ready from the set.
            while let Some((start, end)) = self.next_provable_range()? {
                // TODO: consider handing off to a separate task
                info!("Ready to aggregate complete batch in range: ({start}-{end})");
                self.aggregate_range(start, end).await?;
            }
        }

        Ok(())
    }

    /// Calculate the next provable range bounded by batch size.
    /// If a complete batch exists then remove those entries from `pending`, advance the cursor, and return the range.
    /// Note: the start and end range indices are inclusive.
    fn next_provable_range(&mut self) -> Result<Option<(u64, u64)>> {
        debug!("trying to accumulate next provable range");
        if self.batch_size == 0 {
            return Ok(None);
        }

        // TODO: this initialisation code using the Option<u64> is just for testing.
        // ideally we can persist this somewhere (db), this just allows me to test locally quickly
        // If we don't have a cursor yet, initialize it from the smallest pending height.
        if self.next_expected.is_none() {
            debug!("next expected is not set, trying to set...");
            match self.pending.first() {
                Some(h) => self.next_expected = Some(h.height()),
                None => return Ok(None), // nothing pending yet
            }
        }

        let start = self.next_expected.unwrap();
        let end = start + (self.batch_size as u64) - 1;

        // Walk the ordered set from `start` and ensure we have exactly `batch_size` elements.
        let mut cursor = start;
        let iter = self.pending.range(BlockProofCommitted(start)..);
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
            self.pending.remove(&BlockProofCommitted(h));
        }

        self.next_expected = Some(cursor);
        Ok(Some((start, end)))
    }

    /// Aggregates a range of block proofs, start and end inclusive.
    async fn aggregate_range(&mut self, start: u64, end: u64) -> Result<()> {
        let block_proofs = self.storage.get_block_proofs_in_range(start, end).await?;
        let inner = self.cfg().inner.get("block-exec").unwrap();
        let vkeys = vec![inner.digest; block_proofs.len()];

        let mut public_values = Vec::with_capacity(block_proofs.len());
        let mut proofs = Vec::with_capacity(block_proofs.len());
        for stored_proof in block_proofs {
            let proof: SP1Proof = bincode::deserialize(&stored_proof.proof_data)?;

            public_values.push(stored_proof.public_values);
            proofs.push(ProofInput::new(proof, (*inner.vk).clone()));
        }

        let input = (BlockRangeExecInput { vkeys, public_values }, proofs);

        // NOTE: temporarily to allow local testing in mock mode
        // let prover = ProverClient::builder().mock().build();
        // let res = prover
        //     .prove(&self.cfg().pk(), &self.build_stdin(input)?)
        //     .deferred_proof_verification(false)
        //     .run()?;

        // let output = self.post_process(res)?;

        let (res, output) = self.prove(input).await?;
        self.storage.store_range_proof(start, end, &res, &output).await?;
        info!("Successfully run range prover with result: {output}");

        Ok(())
    }
}
