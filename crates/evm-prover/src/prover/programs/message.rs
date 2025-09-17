//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use alloy_primitives::{Address, FixedBytes};
use alloy_provider::{ProviderBuilder, WsConnect};
use alloy_rpc_types::{EIP1186AccountProofResponse, Filter};
use anyhow::Result;
use evm_hyperlane_types_sp1::{HyperlaneMessageInputs, HyperlaneMessageOutputs};
use evm_state_queries::hyperlane::indexer::HyperlaneIndexer;
use evm_state_types::events::Dispatch;
use evm_storage_proofs::{
    client::EvmClient,
    types::{HyperlaneBranchProof, HyperlaneBranchProofInputs, HYPERLANE_MERKLE_TREE_KEYS},
};
use reqwest::Url;
use sp1_sdk::{include_elf, EnvProver, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin};
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use tokio::time::sleep;

use crate::prover::{
    programs::{types::DefaultProvider, StateQueryProvider},
    ProgramProver, ProverConfig,
};

const TIMEOUT: u64 = 6; // in seconds
const DISTANCE_TO_HEAD: u64 = 32; // in blocks

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

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
    pub prover: EnvProver,
    pub message_store: Arc<HyperlaneMessageStore>,
    pub snapshot_store: Arc<HyperlaneSnapshotStore>,
    pub state_query_provider: Arc<dyn StateQueryProvider>,
}

impl ProgramProver for HyperlaneMessageProver {
    type Input = HyperlaneMessageInputs;
    type Output = HyperlaneMessageOutputs;

    fn cfg(&self) -> &ProverConfig {
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

    fn prover(&self) -> &EnvProver {
        &self.prover
    }
}

impl HyperlaneMessageProver {
    pub fn new(
        app: AppContext,
        message_store: Arc<HyperlaneMessageStore>,
        snapshot_store: Arc<HyperlaneSnapshotStore>,
        state_query_provider: Arc<dyn StateQueryProvider>,
    ) -> Result<Arc<Self>> {
        let config = HyperlaneMessageProver::default_config();
        let prover = ProverClient::from_env();

        Ok(Arc::new(Self {
            app,
            config,
            prover,
            message_store,
            snapshot_store,
            state_query_provider,
        }))
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_HYPERLANE_ELF,
            proof_mode: SP1ProofMode::Compressed,
        }
    }

    /// Run the message prover with indexer
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let evm_provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str(&self.app.evm_rpc).unwrap());
        let evm_client = EvmClient::new(evm_provider.clone());
        let socket = WsConnect::new(&self.app.evm_ws);
        let contract_address = self.app.mailbox_address;
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        let mut indexer = HyperlaneIndexer::new(socket, contract_address, filter.clone());

        loop {
            // get the trusted height and state root from the state query provider
            let height_on_chain = self.state_query_provider.get_height().await;
            let state_root_on_chain = self
                .state_query_provider
                .get_state_root(height_on_chain)
                .await
                .expect("Failed to get state root");

            let proof = evm_client
                .get_proof(
                    &HYPERLANE_MERKLE_TREE_KEYS,
                    self.app.merkle_tree_address,
                    Some(height_on_chain),
                )
                .await
                .expect("Failed to get merkle proof");

            println!("[INFO] state_root_on_chain: {state_root_on_chain}, height_on_chain: {height_on_chain}, trusted height: {}", self.app.merkle_tree_state.read().unwrap().height + 1);

            if self.app.merkle_tree_state.read().unwrap().height >= height_on_chain {
                println!(
                    "[INFO] Waiting for more blocks to occur {}/{}...",
                    height_on_chain,
                    self.app.merkle_tree_state.read().unwrap().height + DISTANCE_TO_HEAD
                );
                sleep(Duration::from_secs(TIMEOUT)).await;
                continue;
            }

            // Check if the root has changed for our height, if so panic
            let new_root_on_chain = evm_client.get_state_root(height_on_chain).await.unwrap();
            if new_root_on_chain != hex::encode(state_root_on_chain) {
                panic!(
                    "The state root has changed at depth HEAD-{DISTANCE_TO_HEAD}, this should not happen! Expected: {state_root_on_chain}, Got: {new_root_on_chain}",
                );
            }

            if let Err(e) = self
                .run_inner(
                    &evm_provider,
                    &mut indexer,
                    height_on_chain,
                    proof.clone(),
                    state_root_on_chain,
                )
                .await
            {
                println!(
                    "Failed to generate proof, Stored Value: {}",
                    hex::encode(proof.storage_proof.last().unwrap().value.to_be_bytes::<32>())
                );
                panic!("[ERROR] Failed to generate proof: {e:?}");
            }
        }
    }

    async fn run_inner(
        self: &Arc<Self>,
        evm_provider: &DefaultProvider,
        indexer: &mut HyperlaneIndexer,
        height_on_chain: u64,
        proof: EIP1186AccountProofResponse,
        state_root_on_chain: FixedBytes<32>,
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
            .to_block(height_on_chain);

        // run the indexer to get all messages that occurred since the last trusted height
        indexer
            .index(self.message_store.clone(), Arc::new(evm_provider.clone()))
            .await
            .expect("Failed to index messages");

        println!(
            "[INFO] Indexed messages, new height {}",
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

        let input = HyperlaneMessageInputs::new(
            state_root_on_chain.to_string(),
            self.app.merkle_tree_address.to_string(),
            messages.clone().into_iter().map(|m| m.message).collect(),
            HyperlaneBranchProofInputs::from(branch_proof),
            snapshot.clone(),
        );

        println!(
            "[INFO] Proving messages with ids: {:?}",
            messages.iter().map(|m| m.message.id()).collect::<Vec<String>>()
        );
        let _proof = self.prove(input).await.expect("Failed to prove");
        println!("[Success] Proof was generated successfully!");

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
            .height = height_on_chain;

        self.app
            .merkle_tree_state
            .write()
            .expect("Failed to write trusted state")
            .snapshot_index += 1;

        Ok(())
    }
}
