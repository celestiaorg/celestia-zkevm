//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use crate::prover::{prover_from_env, RangeProofCommitted, SP1Prover};
use crate::prover::{ProgramProver, ProverConfig};
use alloy::hex::FromHex;
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::{EIP1186AccountProofResponse, Filter};
use anyhow::{Context as AnyhowContext, Result};
use ev_state_queries::{hyperlane::indexer::HyperlaneIndexer, DefaultProvider, StateQueryProvider};
use ev_zkevm_types::events::Dispatch;
use ev_zkevm_types::programs::hyperlane::types::{
    HyperlaneBranchProof, HyperlaneBranchProofInputs, HyperlaneMessageInputs, HyperlaneMessageOutputs,
    HYPERLANE_MERKLE_TREE_KEYS,
};
use reqwest::Url;
use sp1_sdk::{include_elf, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use std::{
    env,
    str::FromStr,
    sync::{Arc, RwLock},
};
use storage::hyperlane::StoredHyperlaneMessage;
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use storage::proofs::ProofStorage;
use tracing::{error, info};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_HYPERLANE_ELF: &[u8] = include_elf!("ev-hyperlane-program");

/// AppContext encapsulates the full set of RPC endpoints and configuration
/// needed to fetch input data for execution and data availability proofs.
///
/// This separates RPC concerns from the proving logic, allowing `AppContext`
/// to be responsible for gathering the data required for the proof system inputs.
pub struct Context {
    // reth http, for example http://127.0.0.1:8545
    pub evm_rpc: String,
    // reth websocket, for example ws://127.0.0.1:8546
    pub evm_ws: String,
    pub mailbox_address: Address,
    pub merkle_tree_address: Address,
    pub merkle_tree_state: RwLock<MerkleTreeState>,
}

/// MerkleTreeState encapsulates the height of the merkle tree in terms of snapshots and blocks
pub struct MerkleTreeState {
    // the index of the snapshot that we will load from the db, initially 0 (empty by default)
    snapshot_index: u64,
    // the index of the last block whose messages were proven, leading up to the snapshot at index snapshot_index
    height: u64,
}

impl MerkleTreeState {
    pub fn new(snapshot_index: u64, height: u64) -> Self {
        Self { snapshot_index, height }
    }
}

/// HyperlaneMessageProver is a prover for generating SP1 proofs for Hyperlane message inclusion in EVM blocks.
pub struct HyperlaneMessageProver {
    pub ctx: Context,
    pub config: ProverConfig,
    pub prover: Arc<SP1Prover>,
    pub message_store: Arc<HyperlaneMessageStore>,
    pub snapshot_store: Arc<HyperlaneSnapshotStore>,
    pub proof_store: Arc<dyn ProofStorage>,
    pub state_query_provider: Arc<dyn StateQueryProvider>,
}

impl ProgramProver for HyperlaneMessageProver {
    type Config = ProverConfig;
    type Input = HyperlaneMessageInputs;
    type Output = HyperlaneMessageOutputs;

    fn cfg(&self) -> &Self::Config {
        &self.config
    }

    fn build_stdin(&self, input: Self::Input) -> Result<SP1Stdin> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);
        Ok(stdin)
    }

    fn post_process(&self, proof: SP1ProofWithPublicValues) -> Result<Self::Output> {
        Ok(bincode::deserialize::<HyperlaneMessageOutputs>(
            proof.public_values.as_slice(),
        )?)
    }

    fn prover(&self) -> Arc<SP1Prover> {
        Arc::clone(&self.prover)
    }
}

impl HyperlaneMessageProver {
    pub fn new(
        ctx: Context,
        message_store: Arc<HyperlaneMessageStore>,
        snapshot_store: Arc<HyperlaneSnapshotStore>,
        proof_store: Arc<dyn ProofStorage>,
        state_query_provider: Arc<dyn StateQueryProvider>,
    ) -> Result<Arc<Self>> {
        let prover = prover_from_env();
        let config = HyperlaneMessageProver::default_config(prover.as_ref());

        Ok(Arc::new(Self {
            ctx,
            config,
            prover,
            message_store,
            snapshot_store,
            proof_store,
            state_query_provider,
        }))
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config(prover: &SP1Prover) -> ProverConfig {
        let (pk, vk) = prover.setup(EV_HYPERLANE_ELF);
        ProverConfig::new(pk, vk, SP1ProofMode::Groth16)
    }

    /// Run the message prover with indexer
    pub async fn run(self: Arc<Self>, mut range_rx: tokio::sync::mpsc::Receiver<RangeProofCommitted>) -> Result<()> {
        let evm_provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str(&self.ctx.evm_rpc).unwrap());
        let socket = WsConnect::new(&self.ctx.evm_ws);
        let contract_address = self.ctx.mailbox_address;
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        let mut indexer = HyperlaneIndexer::new(socket, contract_address, filter.clone());
        loop {
            // Wait for the next range proof to be committed
            let commit_message: RangeProofCommitted =
                range_rx.recv().await.context("Failed to receive commit message")?;

            info!("Received commit message: {:?}", commit_message);

            let committed_height = commit_message.trusted_height;
            let committed_state_root = commit_message.trusted_root;

            let merkle_proof = evm_provider
                .get_proof(
                    self.ctx.merkle_tree_address,
                    HYPERLANE_MERKLE_TREE_KEYS
                        .iter()
                        .map(|k| FixedBytes::from_hex(k).unwrap())
                        .collect(),
                )
                .block_id(committed_height.into())
                .await?;

            info!(
                "state_root: {committed_state_root:?}, height: {committed_height}, trusted height: {}",
                self.ctx.merkle_tree_state.read().unwrap().height + 1
            );

            if let Err(e) = self
                .run_inner(
                    &evm_provider,
                    &mut indexer,
                    committed_height,
                    merkle_proof.clone(),
                    FixedBytes::from_slice(&committed_state_root),
                )
                .await
            {
                error!(
                    "Failed to generate proof, Stored Value: {}",
                    hex::encode(merkle_proof.storage_proof.last().unwrap().value.to_be_bytes::<32>())
                );
                panic!("Failed to generate proof: {e:?}");
            }
        }
    }

    async fn run_inner(
        self: &Arc<Self>,
        evm_provider: &DefaultProvider,
        indexer: &mut HyperlaneIndexer,
        height: u64,
        proof: EIP1186AccountProofResponse,
        state_root: FixedBytes<32>,
    ) -> Result<()> {
        indexer.filter = Filter::new()
            .address(indexer.contract_address)
            .event(&Dispatch::id())
            .from_block(
                self.ctx
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .height
                    + 1,
            )
            .to_block(height);

        // run the indexer to get all messages that occurred since the last trusted height
        indexer
            .index(self.message_store.clone(), Arc::new(evm_provider.clone()))
            .await
            .expect("Failed to index messages");

        // generate a new proof for all messages that occurred since the last trusted height, inserting into the last snapshot
        // then save new snapshot
        let mut snapshot = self
            .snapshot_store
            .get_snapshot(
                self.ctx
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .snapshot_index,
            )
            .expect("Failed to get snapshot");

        let mut messages: Vec<StoredHyperlaneMessage> = Vec::new();
        for block in self
            .ctx
            .merkle_tree_state
            .read()
            .expect("Failed to read trusted state")
            .height
            + 1..=height
        {
            messages.extend(self.message_store.get_by_block(block).expect("Failed to get messages"));
        }

        if messages.is_empty() {
            return Ok(());
        }

        let branch_proof = HyperlaneBranchProof::new(proof);

        // Construct program inputs from values
        let input = HyperlaneMessageInputs::new(
            state_root.to_string(),
            self.ctx.merkle_tree_address.to_string(),
            messages.clone().into_iter().map(|m| m.message).collect(),
            HyperlaneBranchProofInputs::from(branch_proof),
            snapshot.clone(),
        );

        for message in messages.clone() {
            snapshot
                .insert(message.message.id())
                .expect("Failed to insert message into snapshot");
        }

        info!(
            "Proving messages with ids: {:?}",
            messages.iter().map(|m| m.message.id()).collect::<Vec<String>>()
        );

        // Generate a proof
        let proof = self.prove(input).await.expect("Failed to prove");
        self.proof_store
            .store_membership_proof(height, &proof.0, &proof.1)
            .await
            .expect("Failed to store proof");

        info!("Membership proof generated successfully");

        // store snapshot
        self.snapshot_store
            .insert_snapshot(
                self.ctx
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .snapshot_index
                    + 1,
                snapshot,
            )
            .expect("Failed to insert snapshot into snapshot store");

        // update trusted state
        self.ctx
            .merkle_tree_state
            .write()
            .expect("Failed to write trusted state")
            .height = height;

        self.ctx
            .merkle_tree_state
            .write()
            .expect("Failed to write trusted state")
            .snapshot_index += 1;

        Ok(())
    }
}
