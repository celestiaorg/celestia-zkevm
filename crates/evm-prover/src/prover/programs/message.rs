//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use alloy_primitives::{hex::FromHex, Address, FixedBytes};
use alloy_provider::{fillers::FillProvider, Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::Filter;
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

use crate::prover::{ProgramProver, ProverConfig};

pub type DefaultProvider = FillProvider<
    alloy_provider::fillers::JoinFill<
        alloy_provider::Identity,
        alloy_provider::fillers::JoinFill<
            alloy_provider::fillers::GasFiller,
            alloy_provider::fillers::JoinFill<
                alloy_provider::fillers::BlobGasFiller,
                alloy_provider::fillers::JoinFill<
                    alloy_provider::fillers::NonceFiller,
                    alloy_provider::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy_provider::RootProvider,
>;

const FREQUENCY: u64 = 50; // in blocks
const TIMEOUT: u64 = 6; // in seconds
const DISTANCE_TO_HEAD: u64 = 32; // in blocks

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

pub struct AppContext {
    pub celestia_rpc: String,
    // reth http, for example http://127.0.0.1:8545
    pub evm_rpc: String,
    // reth websocket, for example ws://127.0.0.1:8546
    pub evm_ws: String,
    pub mailbox_address: Address,
    pub merkle_tree_address: Address,
    pub trusted_state: RwLock<TrustedState>,
}

pub struct TrustedState {
    // the index of the snapshot that we will load from the db, initially 0 (empty by default)
    snapshot_index: u64,
    // the index of the last message we proofed successfully, initially 0
    height: u64,
}

impl TrustedState {
    pub fn new(snapshot_index: u64, height: u64) -> Self {
        Self { snapshot_index, height }
    }
}

pub struct HyperlaneMessageProver {
    pub app: AppContext,
    pub config: ProverConfig,
    pub prover: EnvProver,
    pub message_store: Arc<HyperlaneMessageStore>,
    pub snapshot_store: Arc<HyperlaneSnapshotStore>,
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

struct ScheduledProofJob {
    height_on_chain: u64,
    state_root_on_chain: FixedBytes<32>,
}

impl HyperlaneMessageProver {
    pub fn new(
        app: AppContext,
        message_store: Arc<HyperlaneMessageStore>,
        snapshot_store: Arc<HyperlaneSnapshotStore>,
    ) -> Result<Arc<Self>> {
        let config = HyperlaneMessageProver::default_config();
        let prover = ProverClient::from_env();

        Ok(Arc::new(Self {
            app,
            config,
            prover,
            message_store,
            snapshot_store,
        }))
    }

    /// Returns the default prover configuration for the block execution program.
    pub fn default_config() -> ProverConfig {
        ProverConfig {
            elf: EVM_HYPERLANE_ELF,
            proof_mode: SP1ProofMode::Compressed,
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let evm_provider: DefaultProvider =
            ProviderBuilder::new().connect_http(Url::from_str(&self.app.evm_rpc).unwrap());
        let evm_client = EvmClient::new(evm_provider.clone());
        let socket = WsConnect::new(&self.app.evm_ws);
        let contract_address = self.app.mailbox_address;
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        let mut indexer = HyperlaneIndexer::new(socket, contract_address, filter.clone());

        loop {
            // todo: get the root and height from celestia instead of directly from reth
            let (state_root_on_chain, height_on_chain) = simulate_get_root_and_height(&evm_provider, &evm_client)
                .await
                .expect("Failed to get root and height");

            println!("[INFO] state_root_on_chain: {state_root_on_chain}, height_on_chain: {height_on_chain}");

            if self.app.trusted_state.read().unwrap().height + FREQUENCY > height_on_chain {
                println!("[INFO] Waiting for more blocks to occur...");
                sleep(Duration::from_secs(TIMEOUT)).await;
                continue;
            }

            indexer.filter = Filter::new()
                .address(indexer.contract_address)
                .event(&Dispatch::id())
                .from_block(
                    self.app
                        .trusted_state
                        .read()
                        .expect("Failed to read trusted state")
                        .height,
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
            // todo: store the proof or directly send it to celestia for verification
            let mut snapshot = self
                .snapshot_store
                .get_snapshot(
                    self.app
                        .trusted_state
                        .read()
                        .expect("Failed to read trusted state")
                        .snapshot_index,
                )
                .expect("Failed to get snapshot");

            let messages = self
                .message_store
                .get_by_block(
                    self.app
                        .trusted_state
                        .read()
                        .expect("Failed to read trusted state")
                        .height,
                )
                .expect("Failed to get messages");

            let proof = evm_client
                .get_proof(
                    &HYPERLANE_MERKLE_TREE_KEYS,
                    self.app.merkle_tree_address,
                    Some(height_on_chain.into()),
                )
                .await
                .expect("Failed to get merkle proof");
            let branch_proof = HyperlaneBranchProof::new(proof);

            let input = HyperlaneMessageInputs::new(
                state_root_on_chain.to_string(),
                self.app.merkle_tree_address.to_string(),
                messages.clone().into_iter().map(|m| m.message).collect(),
                HyperlaneBranchProofInputs::from(branch_proof),
                snapshot.clone(),
            );
            let _proof = self.prove(input).await.expect("Failed to prove");

            // insert messages into snapshot to get new snapshot for next proof
            snapshot
                .insert(messages.iter().map(|m| m.message.id()).collect())
                .expect("Failed to insert messages into snapshot");
            // store snapshot
            self.snapshot_store
                .insert_snapshot(
                    self.app
                        .trusted_state
                        .read()
                        .expect("Failed to read trusted state")
                        .snapshot_index
                        + 1,
                    snapshot,
                )
                .expect("Failed to insert snapshot into snapshot store");

            // update trusted state
            self.app
                .trusted_state
                .write()
                .expect("Failed to write trusted state")
                .height = height_on_chain;

            self.app
                .trusted_state
                .write()
                .expect("Failed to write trusted state")
                .snapshot_index += 1;
        }
    }
}

async fn simulate_get_root_and_height(provider: &DefaultProvider, client: &EvmClient) -> Result<(FixedBytes<32>, u64)> {
    // todo: instead query celestia for a recent state root and height provided by our light client
    let height = provider.get_block_number().await.unwrap() - DISTANCE_TO_HEAD;
    let root = client.get_state_root(height).await.unwrap();
    Ok((FixedBytes::from_hex(&root).unwrap(), height))
}

#[tokio::test]
async fn test_run_prover() {
    #[allow(unused_imports)]
    use super::*;

    let app = AppContext {
        celestia_rpc: "http://127.0.0.1:26657".to_string(),
        evm_rpc: "http://127.0.0.1:8545".to_string(),
        evm_ws: "ws://127.0.0.1:8546".to_string(),
        mailbox_address: Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap(),
        merkle_tree_address: Address::from_str("0xFCb1d485ef46344029D9E8A7925925e146B3430E").unwrap(),
        trusted_state: RwLock::new(TrustedState::new(0, 0)),
    };

    let hyperlane_message_store =
        Arc::new(HyperlaneMessageStore::from_path_relative(2, storage::hyperlane::message::IndexMode::Block).unwrap());
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::from_path_relative(2).unwrap());
    hyperlane_message_store.prune_all().unwrap();
    hyperlane_snapshot_store.prune_all().unwrap();

    let prover = HyperlaneMessageProver::new(app, hyperlane_message_store, hyperlane_snapshot_store).unwrap();
    prover.run().await.unwrap();
}
