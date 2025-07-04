/// This is a test script used for scraping data from the docker compose network in the repository.
///
/// Run this script:
/// ```
/// cargo run -p evm-prover --bin script
/// ```
use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use celestia_types::nmt::{MerkleHash, Namespace};
use nmt_rs::TmSha2Hasher;
use reth_chainspec::ChainSpec;
use rsp_primitives::genesis::Genesis;
use sp1_sdk::ProverClient;

use crate::prover::prover::{AppContext, BlockExecProver, ProverConfig, EVM_EXEC_ELF};

mod commands;
mod config;
mod grpc;
mod proto;
mod prover;

/// Loads the genesis file from disk and converts it into a ChainSpec
fn load_chain_spec_from_genesis(path: &str) -> Result<(Genesis, Arc<ChainSpec>), Box<dyn Error>> {
    let genesis_json = fs::read_to_string(path)?;

    let genesis = Genesis::Custom(genesis_json);
    let chain_spec = Arc::new((&genesis).try_into()?);
    Ok((genesis, chain_spec))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ProverConfig {
        elf: EVM_EXEC_ELF,
        proof_mode: sp1_sdk::SP1ProofMode::Compressed,
    };

    let prover = ProverClient::from_env();

    let genesis_path = env::var("GENESIS_PATH").expect("GENESIS_PATH must be set");
    let (genesis, chain_spec) =
        load_chain_spec_from_genesis(&genesis_path).expect("error loading genesis and chain spec");

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let app = AppContext {
        chain_spec: chain_spec,
        genesis: genesis,
        namespace: namespace,
        celestia_rpc: "http://127.0.0.1:26658".to_string(),
        evm_rpc: "http://127.0.0.1:8545".to_string(),
        sequencer_rpc: "http://127.0.0.1:7331".to_string(),
    };

    let block_prover = BlockExecProver {
        app: app,
        config: config,
        prover: prover,
    };

    let block_number = 30;
    // let block_number = 100;

    let inclusion_height = block_prover.header_inclusion_height(block_number).await?;
    println!(
        "successfully got inclusion height: {} for block number: {}",
        inclusion_height, block_number
    );

    let blob = block_prover.blob_for_height(block_number, inclusion_height).await?;
    println!("successfully got blob with commitment: {:?}", blob.commitment);

    let header = block_prover.extended_header(inclusion_height).await?;
    println!("successfully got header: {}", header.hash());

    let blob_proof = block_prover.blob_inclusion_proof(blob, &header).await?;
    println!("successfully got blob inclusion proof");

    let (data_root, data_root_proof) = block_prover.data_root_proof(&header)?;
    println!("successfully got data root inclusion proof");

    blob_proof.verify(header.dah.hash()).expect("failed to verify proof");

    let hasher = TmSha2Hasher {};
    data_root_proof
        .verify_range(
            &header.header.hash().as_bytes().try_into().unwrap(),
            &[hasher.hash_leaf(&data_root)],
        )
        .expect("failed to verify header proof");

    println!("verified proofs");

    Ok(())
}
