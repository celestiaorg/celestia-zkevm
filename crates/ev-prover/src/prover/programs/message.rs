//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use crate::prover::ProverConfig;
use alloy_primitives::{hex::FromHex, Address, FixedBytes};
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::{EIP1186AccountProofResponse, Filter};
use anyhow::{Context, Result};
use ev_state_queries::{hyperlane::indexer::HyperlaneIndexer, DefaultProvider, StateQueryProvider};
use ev_zkevm_types::programs::hyperlane::types::{
    HyperlaneBranchProof, HyperlaneBranchProofInputs, HyperlaneMessageInputs, HyperlaneMessageOutputs,
};
use ev_zkevm_types::{events::Dispatch, programs::hyperlane::types::HYPERLANE_MERKLE_TREE_KEYS};
use reqwest::Url;

#[cfg(feature = "sp1")]
use sp1_sdk::include_elf;

use crate::proof_system::{ProofMode, ProofSystemBackend, ProverFactory};
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use storage::proofs::ProofStorage;
use tokio::time::sleep;
use tracing::{debug, error, info};

const TIMEOUT: u64 = 6; // in seconds
const DISTANCE_TO_HEAD: u64 = 32; // in blocks

// Program IDs for different proof systems
#[cfg(feature = "sp1")]
pub const EV_HYPERLANE_PROGRAM_ID: &[u8] = include_elf!("ev-hyperlane-program");

// NOTE: RISC0 ImageID would be loaded from ev-hyperlane-host, but that crate
// is excluded from workspace due to crypto patch conflicts.
// For RISC0 support, the ImageID must be provided via ProverConfig.
#[cfg(all(feature = "risc0", not(feature = "sp1")))]
pub const EV_HYPERLANE_PROGRAM_ID: &[u8] = &[]; // Placeholder - use config.program_id instead

// Compatibility alias
pub const EV_HYPERLANE_ELF: &[u8] = EV_HYPERLANE_PROGRAM_ID;

/// AppContext encapsulates the full set of RPC endpoints and configuration
/// needed to fetch input data for execution and data availability proofs.
///
/// This separates RPC concerns from the proving logic, allowing `AppContext`
/// to be responsible for gathering the data required for the proof system inputs.
pub struct AppContext {
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
    pub app: AppContext,
    pub config: ProverConfig,
    pub prover: Arc<dyn ProofSystemBackend>,
    pub message_store: Arc<HyperlaneMessageStore>,
    pub snapshot_store: Arc<HyperlaneSnapshotStore>,
    pub proof_store: Arc<dyn ProofStorage>,
    pub state_query_provider: Arc<dyn StateQueryProvider>,
}

// ProgramProver trait removed - using ProofSystemBackend directly

impl HyperlaneMessageProver {
    pub fn new(
        app: AppContext,
        message_store: Arc<HyperlaneMessageStore>,
        snapshot_store: Arc<HyperlaneSnapshotStore>,
        proof_store: Arc<dyn ProofStorage>,
        state_query_provider: Arc<dyn StateQueryProvider>,
    ) -> Result<Arc<Self>> {
        let config = HyperlaneMessageProver::default_config();
        let prover = Arc::from(ProverFactory::from_env()?);

        Ok(Arc::new(Self {
            app,
            config,
            prover,
            message_store,
            snapshot_store,
            proof_store,
            state_query_provider,
        }))
    }

    /// Returns the default prover configuration for the Hyperlane message program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            program_id: EV_HYPERLANE_PROGRAM_ID,
            proof_mode: ProofMode::Groth16,
        }
    }

    /// Generates a proof for the given Hyperlane message input using the configured proof system.
    async fn prove(
        &self,
        input: HyperlaneMessageInputs,
    ) -> Result<(crate::proof_system::UnifiedProof, HyperlaneMessageOutputs)> {
        // Serialize input to bytes
        let input_bytes = bincode::serialize(&input)?;

        // Use program ID from config
        let program_id = self.config.program_id;
        let proof_mode = self.config.proof_mode;

        // Generate proof using the abstraction layer
        let proof = self.prover.prove(program_id, &input_bytes, proof_mode).await?;

        // Deserialize public values to output
        let output: HyperlaneMessageOutputs = bincode::deserialize(&proof.public_values)?;

        Ok((proof, output))
    }

    /// Run the message prover with indexer
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let evm_provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str(&self.app.evm_rpc).unwrap());
        let socket = WsConnect::new(&self.app.evm_ws);
        let contract_address = self.app.mailbox_address;
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        let mut indexer = HyperlaneIndexer::new(socket, contract_address, filter.clone());

        loop {
            // get the trusted height and state root from the state query provider
            let height = self.state_query_provider.get_height().await;
            let state_root = self
                .state_query_provider
                .get_state_root(height)
                .await
                .expect("Failed to get state root");

            let merkle_proof = evm_provider
                .get_proof(
                    self.app.merkle_tree_address,
                    HYPERLANE_MERKLE_TREE_KEYS
                        .iter()
                        .map(|k| FixedBytes::from_hex(k).unwrap())
                        .collect(),
                )
                .block_id(height.into())
                .await?;
            info!(
                "state_root: {state_root}, height: {height}, trusted height: {}",
                self.app.merkle_tree_state.read().unwrap().height + 1
            );

            if self.app.merkle_tree_state.read().unwrap().height >= height {
                info!(
                    "Waiting for more blocks to occur {}/{}...",
                    height,
                    self.app.merkle_tree_state.read().unwrap().height + DISTANCE_TO_HEAD
                );
                sleep(Duration::from_secs(TIMEOUT)).await;
                continue;
            }

            // Check if the root has changed for our height, if so panic
            let block = evm_provider
                .get_block(height.into())
                .await?
                .context("Failed to get block")?;
            // This is an optional check to ensure the state root is always finalized
            let new_root = alloy::hex::encode(block.header.state_root);
            if new_root != hex::encode(state_root) {
                panic!(
                    "The state root has changed at depth HEAD-{DISTANCE_TO_HEAD}, this should not happen! Expected: {state_root}, Got: {new_root}",
                );
            }

            if let Err(e) = self
                .run_inner(&evm_provider, &mut indexer, height, merkle_proof.clone(), state_root)
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
                self.app
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
        debug!(
            "Indexed messages, new height {}",
            self.message_store.current_index().expect("Failed to get current index")
        );

        // generate a new proof for all messages that occurred since the last trusted height, inserting into the last snapshot
        // then save new snapshot
        let mut snapshot = self
            .snapshot_store
            .get_snapshot(
                self.app
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .snapshot_index,
            )
            .expect("Failed to get snapshot");
        let messages = self
            .message_store
            .get_by_block(
                self.app
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .height
                    + 1,
            )
            .expect("Failed to get messages");
        let branch_proof = HyperlaneBranchProof::new(proof);

        // Construct program inputs from values
        let input = HyperlaneMessageInputs::new(
            state_root.to_string(),
            self.app.merkle_tree_address.to_string(),
            messages.clone().into_iter().map(|m| m.message).collect(),
            HyperlaneBranchProofInputs::from(branch_proof),
            snapshot.clone(),
        );
        info!(
            "Proving messages with ids: {:?}",
            messages.iter().map(|m| m.message.id()).collect::<Vec<String>>()
        );

        // Generate a proof
        let (proof, output) = self.prove(input).await.expect("Failed to prove");

        // UnifiedProof already contains proof_bytes and public_values
        let proof_data = proof.proof_bytes.clone();
        let public_values = proof.public_values.clone();

        self.proof_store
            .store_membership_proof(
                height,
                storage::proofs::ProofSystem::SP1,
                &proof_data,
                &public_values,
                &output,
            )
            .await
            .expect("Failed to store proof");

        // insert messages into snapshot to get new snapshot for next proof
        for message in messages {
            snapshot
                .insert(message.message.id())
                .expect("Failed to insert messages into snapshot");
        }

        // store snapshot
        self.snapshot_store
            .insert_snapshot(
                self.app
                    .merkle_tree_state
                    .read()
                    .expect("Failed to read trusted state")
                    .snapshot_index
                    + 1,
                snapshot,
            )
            .expect("Failed to insert snapshot into snapshot store");

        // update trusted state
        self.app
            .merkle_tree_state
            .write()
            .expect("Failed to write trusted state")
            .height = height;

        self.app
            .merkle_tree_state
            .write()
            .expect("Failed to write trusted state")
            .snapshot_index += 1;

        Ok(())
    }
}
