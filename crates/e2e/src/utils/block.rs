// This endpoint generates a block proof for a range (trusted_height, target_height)
// and wraps it recursively into a single groth16 proof using the ev-range-exec program.

use alloy_primitives::{FixedBytes, hex};
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::Blob;
use celestia_types::nmt::{Namespace, NamespaceProof};
use ev_prover::prover::programs::combined::get_block_inputs;
use ev_prover::{generate_client_executor_input, get_sequencer_pubkey, load_chain_spec_from_genesis};
use ev_types::v1::SignedData;
use ev_zkevm_types::programs::block::{BlockExecOutput, BlockRangeExecInput, BlockRangeExecOutput};
use eyre::Context;
use prost::Message;
use rsp_client_executor::io::EthClientExecutorInput;
use sp1_sdk::{EnvProver, HashableKey, SP1Proof, SP1ProofWithPublicValues, SP1Stdin};
use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use storage::proofs::{ProofStorage, RocksDbProofStorage};
use tokio::task::JoinHandle;
use tracing::debug;

use crate::utils::rpc_config;

pub async fn prove_blocks(
    start_height: u64,
    trusted_height: u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
    client: Arc<EnvProver>,
) -> Result<SP1ProofWithPublicValues, Box<dyn Error>> {
    let mut trusted_height = trusted_height;
    let prover_mode = env::var("SP1_PROVER").unwrap_or("cpu".to_string());
    let proof = {
        // parallel mode (network)
        if prover_mode == "network" {
            parallel_prover(start_height, &mut trusted_height, num_blocks, trusted_root, client).await?
        }
        // mock mode is not possible for recursive groth16 proofs
        else if prover_mode == "mock" {
            panic!("Recursive groth16 proofs are not supported in mock mode");
        }
        // synchronous mode (cuda, cpu)
        else {
            synchronous_prover(start_height, &mut trusted_height, num_blocks, trusted_root, client).await?
        }
    };
    Ok(proof)
}

pub async fn parallel_prover(
    start_height: u64,
    trusted_height: &mut u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
    client: Arc<EnvProver>,
) -> Result<SP1ProofWithPublicValues, Box<dyn Error>> {
    let storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("data")
        .join("proofs.db");
    let proof_storage = Arc::new(RocksDbProofStorage::new(storage_path)?);

    let genesis_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("config")
        .join("genesis.json");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(genesis_path.to_str().unwrap())?;

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let celestia_client = Client::new(rpc_config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;
    let pub_key = get_sequencer_pubkey(rpc_config::SEQUENCER_URL.to_string()).await?;

    let block_prover_elf = fs::read("elfs/ev-exec-elf").expect("Failed to read ELF");
    let (pk, vk) = (*client).setup(&block_prover_elf);

    let mut trusted_heights = Vec::new();
    let mut trusted_roots = Vec::new();
    trusted_heights.push(*trusted_height);
    trusted_roots.push(*trusted_root);

    // before we generate proofs in parallel mode, we execute all blocks to
    // collect the trusted height and root to then supply them optimistically to the prover
    for block_number in start_height..=(start_height + num_blocks) {
        debug!("\nProcessing block: {block_number}");
        let blobs: Vec<Blob> = celestia_client
            .blob_get_all(block_number, &[namespace])
            .await
            .expect("Failed to get blobs")
            .unwrap_or_default();
        debug!("Got {} blobs for block: {}", blobs.len(), block_number);

        let extended_header = celestia_client
            .header_get_by_height(block_number)
            .await
            .expect("Failed to get extended header");
        let namespace_data = celestia_client
            .share_get_namespace_data(&extended_header, namespace)
            .await
            .expect("Failed to get namespace data");
        let mut proofs: Vec<NamespaceProof> = Vec::new();
        for row in namespace_data.rows {
            proofs.push(row.proof);
        }
        debug!("Got NamespaceProofs, total: {}", proofs.len());

        let mut executor_inputs: Vec<EthClientExecutorInput> = Vec::new();
        let mut last_height = *trusted_height;
        for blob in blobs.as_slice() {
            let data = match SignedData::decode(blob.data.as_slice()) {
                Ok(data) => data.data.unwrap(),
                Err(_) => continue,
            };
            let height = data.metadata.unwrap().height;
            last_height = height;
            debug!("Got SignedData for EVM block {height}");

            let client_executor_input =
                generate_client_executor_input(rpc_config::EVM_RPC_URL, height, chain_spec.clone(), genesis.clone())
                    .await
                    .expect("Failed to generate client executor input");
            executor_inputs.push(client_executor_input);
        }

        let provider = ProviderBuilder::new().connect_http(rpc_config::EVM_RPC_URL.parse()?);
        let block: alloy_rpc_types::Block = provider
            .get_block_by_number(last_height.into())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block {} not found", last_height))?;

        trusted_heights.push(last_height);
        trusted_roots.push(block.header.state_root.into());
    }

    // now we can generate proofs in parallel
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    for block_number in start_height..=(start_height + num_blocks) {
        let handle = tokio::spawn({
            let celestia_client = Client::new(rpc_config::CELESTIA_RPC_URL, None)
                .await
                .context("Failed creating Celestia RPC client")?;
            let chain_spec = chain_spec.clone();
            let genesis = genesis.clone();
            let pub_key = pub_key.clone();
            let pk = pk.clone();
            let proof_storage = proof_storage.clone();
            let mut trusted_heights = trusted_heights.clone();
            let mut trusted_roots = trusted_roots.clone();
            // spawn a tokio task for each block proof and join them to await all proofs before
            // wrapping them in groth16
            let client_clone = client.clone();
            async move {
                debug!("\nProcessing block: {block_number}");
                let mut stdin = SP1Stdin::new();
                let inputs = get_block_inputs(
                    &celestia_client,
                    rpc_config::EVM_RPC_URL,
                    block_number,
                    namespace,
                    &mut trusted_heights[(block_number - start_height) as usize],
                    &mut trusted_roots[(block_number - start_height) as usize],
                    chain_spec.clone(),
                    genesis.clone(),
                    pub_key.clone(),
                )
                .await
                .expect("Failed to write inputs");
                stdin.write(&inputs);
                debug!("Generating proof for block: {block_number}");
                let proof = client_clone
                    .prove(&pk, &stdin)
                    .compressed()
                    .run()
                    .expect("failed to generate proof");

                // store proof in proof storage
                // later retrieved to generate range proof
                proof_storage
                    .store_block_proof(
                        block_number,
                        &proof,
                        &bincode::deserialize(proof.public_values.as_slice())
                            .expect("Failed to deserialize public values"),
                    )
                    .await
                    .expect("Failed to store proof");
                debug!("Proof stored successfully!");
            }
        });

        handles.push(handle);
    }

    // wait for all block proofs to be generated before
    // continuing with the range proof
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // reinitialize the prover client
    let mut stdin = SP1Stdin::new();
    let range_prover_elf = fs::read("elfs/ev-range-exec-elf").expect("Failed to read ELF");
    let (pk, _) = client.clone().setup(&range_prover_elf);

    // load all proofs from storage for range proof
    debug!("Loading block proofs from storage for range proof");
    let block_proofs = proof_storage
        .get_block_proofs_in_range(start_height, start_height + num_blocks - 1)
        .await?;
    debug!(
        "Loaded {} block proofs from storage for range proof",
        block_proofs.len()
    );

    let vkeys = vec![vk.hash_u32(); block_proofs.len()];

    let public_inputs = block_proofs
        .iter()
        .map(|proof| proof.public_values.to_vec())
        .collect::<Vec<_>>();

    let input = BlockRangeExecInput {
        vkeys,
        public_values: public_inputs,
    };
    stdin.write(&input);

    debug!("Writing block proofs to stdin for range proof");
    for block_proof in &block_proofs {
        let proof_deserialized: SP1Proof = bincode::deserialize(block_proof.proof_data.as_slice())?;
        let SP1Proof::Compressed(ref proof) = proof_deserialized else {
            panic!()
        };
        stdin.write_proof(*proof.clone(), vk.vk.clone());
    }
    debug!("Wrote block proofs to stdin for range proof");

    // finally generate the range proof and return it
    let proof = client
        .clone()
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");

    let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
    debug!(
        "Final EVM state height: {:?} and root: {:?}, which should be used for proving messages using ./message.rs",
        public_values.new_height, public_values.new_state_root
    );

    Ok(proof)
}

pub async fn synchronous_prover(
    start_height: u64,
    trusted_height: &mut u64,
    num_blocks: u64,
    trusted_root: &mut FixedBytes<32>,
    client: Arc<EnvProver>,
) -> Result<SP1ProofWithPublicValues, Box<dyn Error>> {
    let genesis_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("config")
        .join("genesis.json");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(genesis_path.to_str().unwrap())?;
    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;
    let celestia_client = Client::new(rpc_config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;
    let pub_key = get_sequencer_pubkey(rpc_config::SEQUENCER_URL.to_string()).await?;
    let block_prover_elf = fs::read("elfs/ev-exec-elf").expect("Failed to read ELF");
    let (pk, vk) = client.clone().setup(&block_prover_elf);

    let mut block_proofs: Vec<SP1ProofWithPublicValues> = Vec::new();
    // loop and adjust trusted state for each iteration,
    // collect all proofs into a vec and return
    // later wrap them in groth16
    for block_number in start_height..=(start_height + num_blocks) {
        let mut stdin = SP1Stdin::new();
        let inputs = get_block_inputs(
            &celestia_client,
            rpc_config::EVM_RPC_URL,
            block_number,
            namespace,
            &mut *trusted_height,
            &mut *trusted_root,
            chain_spec.clone(),
            genesis.clone(),
            pub_key.clone(),
        )
        .await
        .expect("Failed to write inputs");
        stdin.write(&inputs);

        debug!("Generating proof for block: {block_number}, trusted height: {trusted_height}");
        let proof = client
            .clone()
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .expect("failed to generate proof");
        block_proofs.push(proof.clone());
        debug!("Proof generated successfully!");

        let public_values: BlockExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
        // update trusted root and height
        *trusted_root = public_values.new_state_root.into();
        *trusted_height = public_values.new_height;
        debug!("New state root: {:?}", *trusted_root);
    }

    // reinitialize the prover client
    let mut stdin = SP1Stdin::new();
    let range_prover_elf = fs::read("elfs/ev-range-exec-elf").expect("Failed to read ELF");
    let (pk, _) = client.clone().setup(&range_prover_elf);

    let vkeys = vec![vk.hash_u32(); block_proofs.len()];

    let public_inputs = block_proofs
        .iter()
        .map(|proof| proof.public_values.to_vec())
        .collect::<Vec<_>>();

    let input = BlockRangeExecInput {
        vkeys,
        public_values: public_inputs,
    };
    stdin.write(&input);

    for block_proof in &block_proofs {
        let SP1Proof::Compressed(ref proof) = block_proof.proof else {
            panic!()
        };
        stdin.write_proof(*proof.clone(), vk.vk.clone());
    }

    let proof = client
        .clone()
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");

    let public_values: BlockRangeExecOutput = bincode::deserialize(proof.public_values.as_slice())?;
    debug!("Target state root: {:?}", public_values.new_state_root);

    Ok(proof)
}
