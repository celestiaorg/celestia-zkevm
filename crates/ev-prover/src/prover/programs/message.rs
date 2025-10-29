//! The prover implementation of the Hyperlane Message circuit proves all messages that have occurred in between
//! two given heights against a given EVM block height.

#![allow(dead_code)]
use crate::prover::{prover_from_env, RangeProofCommitted, SP1Prover};
use crate::prover::{ProgramProver, ProverConfig};
use crate::ISM_ID;
use alloy::hex::FromHex;
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::{EIP1186AccountProofResponse, Filter};
use anyhow::{Context as AnyhowContext, Result};
use celestia_grpc_client::{CelestiaIsmClient, MsgProcessMessage, MsgSubmitMessages};
use ev_state_queries::{hyperlane::indexer::HyperlaneIndexer, DefaultProvider, StateQueryProvider};
use ev_zkevm_types::events::Dispatch;
use ev_zkevm_types::hyperlane::encode_hyperlane_message;
use ev_zkevm_types::programs::hyperlane::types::{
    HyperlaneBranchProof, HyperlaneBranchProofInputs, HyperlaneMessageInputs, HyperlaneMessageOutputs,
    HYPERLANE_MERKLE_TREE_KEYS,
};
use reqwest::Url;
use sp1_sdk::{include_elf, SP1ProofMode, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use std::{env, str::FromStr, sync::Arc};
use storage::hyperlane::StoredHyperlaneMessage;
use storage::hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore};
use storage::proofs::ProofStorage;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_HYPERLANE_ELF: &[u8] = include_elf!("ev-hyperlane-program");

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
}

#[derive(Clone)]
pub struct MessageProverConfig {
    pub pk: Arc<SP1ProvingKey>,
    pub vk: Arc<SP1VerifyingKey>,
    pub proof_mode: SP1ProofMode,
}

impl MessageProverConfig {
    pub fn new(pk: SP1ProvingKey, vk: SP1VerifyingKey, mode: SP1ProofMode) -> Self {
        MessageProverConfig {
            pk: Arc::new(pk),
            vk: Arc::new(vk),
            proof_mode: mode,
        }
    }
}

impl ProverConfig for MessageProverConfig {
    fn pk(&self) -> Arc<SP1ProvingKey> {
        Arc::clone(&self.pk)
    }

    fn vk(&self) -> Arc<SP1VerifyingKey> {
        Arc::clone(&self.vk)
    }

    fn proof_mode(&self) -> SP1ProofMode {
        self.proof_mode
    }
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
    pub ctx: AppContext,
    pub config: MessageProverConfig,
    pub prover: Arc<SP1Prover>,
    pub message_store: Arc<HyperlaneMessageStore>,
    pub snapshot_store: Arc<HyperlaneSnapshotStore>,
    pub proof_store: Arc<dyn ProofStorage>,
    pub state_query_provider: Arc<dyn StateQueryProvider>,
}

impl ProgramProver for HyperlaneMessageProver {
    type Config = MessageProverConfig;
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
        ctx: AppContext,
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
    pub fn default_config(prover: &SP1Prover) -> MessageProverConfig {
        let (pk, vk) = prover.setup(EV_HYPERLANE_ELF);
        MessageProverConfig::new(pk, vk, SP1ProofMode::Groth16)
    }

    /// Run the message prover with indexer
    pub async fn run(
        self: Arc<Self>,
        mut range_rx: Receiver<RangeProofCommitted>,
        ism_client: Arc<CelestiaIsmClient>,
        is_proving_messages: Arc<Mutex<bool>>,
    ) -> Result<()> {
        let evm_provider: DefaultProvider = ProviderBuilder::new().connect_http(Url::from_str(&self.ctx.evm_rpc)?);
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
                        .map(|k| FixedBytes::from_hex(k).expect("Failed to parse fixed bytes"))
                        .collect(),
                )
                .block_id(committed_height.into())
                .await?;

            if let Err(e) = self
                .run_inner(
                    &evm_provider,
                    &mut indexer,
                    committed_height,
                    merkle_proof.clone(),
                    FixedBytes::from_slice(&committed_state_root),
                    &ism_client,
                )
                .await
            {
                error!(
                    "Failed to generate proof, Stored Value: {}, error: {e:?}",
                    hex::encode(
                        merkle_proof
                            .storage_proof
                            .last()
                            .ok_or(anyhow::anyhow!("No storage proof"))?
                            .value
                            .to_be_bytes::<32>()
                    )
                );
            }
            *is_proving_messages.lock().await = false;
        }
    }

    async fn run_inner(
        self: &Arc<Self>,
        evm_provider: &DefaultProvider,
        indexer: &mut HyperlaneIndexer,
        height: u64,
        proof: EIP1186AccountProofResponse,
        state_root: FixedBytes<32>,
        ism_client: &CelestiaIsmClient,
    ) -> Result<()> {
        // generate a new proof for all messages that occurred since the last trusted height, inserting into the last snapshot
        // then save new snapshot
        let mut snapshot = self.snapshot_store.get_snapshot(self.snapshot_store.current_index()?)?;
        if snapshot.height == height {
            debug!("No new ev blocks so no new messages to prove");
            return Ok(());
        }
        let start_height = snapshot.height + 1;

        indexer.filter = Filter::new()
            .address(indexer.contract_address)
            .event(&Dispatch::id())
            .from_block(start_height)
            .to_block(height);

        // run the indexer to get all messages that occurred since the last trusted height
        indexer
            .index(self.message_store.clone(), Arc::new(evm_provider.clone()))
            .await?;

        let mut messages: Vec<StoredHyperlaneMessage> = Vec::new();
        for block in start_height..=height {
            messages.extend(self.message_store.get_by_block(block)?);
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
            snapshot.tree.clone(),
        );

        for message in messages.clone() {
            snapshot.tree.insert(message.message.id())?;
        }

        info!(
            "Proving messages with ids: {:?}",
            messages.iter().map(|m| m.message.id()).collect::<Vec<String>>()
        );

        // Prove messages against trusted root
        let message_proof = self.prove(input).await?;
        info!("Message proof generated successfully");

        let message_proof_msg = MsgSubmitMessages::new(
            ISM_ID.to_string(),
            height,
            message_proof.0.bytes(),
            message_proof.0.public_values.as_slice().to_vec(),
            ism_client.signer_address().to_string(),
        );

        info!("Submitting Hyperlane tree proof to ZKISM...");
        let response = ism_client.send_tx(message_proof_msg).await?;

        assert!(response.success);
        info!("[Done] ZKISM was updated successfully");

        info!("Relaying verified Hyperlane messages to Celestia...");
        // submit all now verified messages to hyperlane
        for message in messages.clone() {
            let message_hex = alloy::hex::encode(encode_hyperlane_message(&message.message)?);
            let msg = MsgProcessMessage::new(
                // Celestia mailbox id, todo: add to config
                "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string(),
                ism_client.signer_address().to_string(),
                alloy::hex::encode(vec![]), // empty metadata; messages are pre-authorized before submission
                message_hex,
            );
            let response = ism_client.send_tx(msg).await?;
            assert!(response.success);
        }
        info!("[Done] Tia was bridged back to Celestia");

        self.proof_store
            .store_membership_proof(height, &message_proof.0, &message_proof.1)
            .await?;

        snapshot.height = height;
        self.snapshot_store
            .insert_snapshot(self.snapshot_store.current_index()? + 1, snapshot)?;

        Ok(())
    }
}
