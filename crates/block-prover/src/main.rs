use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use eyre::Context;
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer};
use sha3::{Digest, Keccak256};
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

use alloy_provider::ProviderBuilder;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::nmt::{MerkleHash, Namespace};
use celestia_types::{Blob, Commitment, ExtendedHeader, ShareProof};
use eq_common::KeccakInclusionToDataRootProofInput;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::BlockNumber,
};
use nmt_rs::{
    TmSha2Hasher,
    simple_merkle::{db::MemDb, proof::Proof, tree::MerkleTree},
};
use reth_chainspec::ChainSpec;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_rpc_db::RpcDb;
use tendermint::merkle::Hash as TmHash;
use tendermint_proto::{
    Protobuf,
    v0_38::{types::BlockId as RawBlockId, version::Consensus as RawConsensusVersion},
};

mod config {
    pub const CELESTIA_RPC_URL: &str = "http://localhost:26658";
    pub const EVM_RPC_URL: &str = "http://localhost:8545";
    pub const INDEXER_URL: &str = "http://localhost:8080";
}

/// Loads the genesis file from disk and converts it into a ChainSpec
fn load_chain_spec_from_genesis(path: &str) -> Result<(Genesis, Arc<ChainSpec>), Box<dyn Error>> {
    let genesis_json = fs::read_to_string(path).wrap_err_with(|| format!("Failed to read genesis file at {}", path))?;

    let genesis = Genesis::Custom(genesis_json);
    let chain_spec = Arc::new((&genesis).try_into()?);
    Ok((genesis, chain_spec))
}

/// Generates the ClientExecutorInput and serializes it to a bincode file
async fn generate_and_write_stf(
    rpc_url: &str,
    block_number: u64,
    chain_spec: Arc<ChainSpec>,
    genesis: Genesis,
    path: &str,
) -> Result<(), Box<dyn Error>> {
    let host_executor = EthHostExecutor::eth(chain_spec.clone(), None);

    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let rpc_db = RpcDb::new(provider.clone(), block_number - 1);

    let client_input = host_executor
        .execute(block_number, &rpc_db, &provider, genesis, None, false)
        .await
        .wrap_err_with(|| format!("Failed to execute block {}", block_number))?;

    let encoded = bincode::serialize(&client_input).wrap_err("Failed to serialize client input to bincode")?;

    fs::write(path, &encoded).wrap_err("Failed to write encoded client input to file")?;
    Ok(())
}

fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    general_purpose::STANDARD.decode(&s).map_err(serde::de::Error::custom)
}

/// Response structure for the inclusion height API
#[derive(Debug, Deserialize)]
struct InclusionHeightResponse {
    eth_block_number: u64,
    celestia_height: u64,
    #[serde(deserialize_with = "deserialize_base64")]
    blob_commitment: Vec<u8>,
}

pub async fn get_data_availability_commitment(indexer_url: &str, evm_block_height: u64) -> Result<(u64, Vec<u8>)> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

    let url = format!(
        "{}/inclusion_height/{}",
        indexer_url.trim_end_matches('/'),
        evm_block_height
    );

    // Retry strategy: exponential backoff starting at 500ms, up to 10 retries
    let backoff = ExponentialBackoff::from_millis(500)
        .max_delay(Duration::from_secs(10))
        .map(jitter)
        .take(10);

    Retry::spawn(backoff, || {
        let client = client.clone();
        let url = url.clone();

        async move {
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| anyhow!("Request failed: {}", e))?;

            match response.status() {
                StatusCode::OK => {
                    let data: InclusionHeightResponse = response
                        .json()
                        .await
                        .map_err(|e| anyhow!("Failed to parse response JSON: {}", e))?;

                    if data.eth_block_number != evm_block_height {
                        return Err(anyhow!(
                            "Sanity check failed: expected {}, got {}",
                            evm_block_height,
                            data.eth_block_number
                        ));
                    }

                    Ok((data.celestia_height, data.blob_commitment))
                }
                status => {
                    let body = response.text().await.unwrap_or_else(|_| "<failed to read body>".into());
                    Err(anyhow!("Unexpected status code {}: {}", status, body))
                }
            }
        }
    })
    .await
}

pub async fn fetch_blob_and_header(
    client: &Client,
    indexer_url: &str,
    block_number: u64,
    namespace: Namespace,
) -> Result<(Blob, ExtendedHeader, Commitment)> {
    let (inclusion_height, raw_commitment) = get_data_availability_commitment(indexer_url, block_number).await?;

    let hash: TmHash = raw_commitment[..raw_commitment.len()].try_into().unwrap();
    let commitment = Commitment::new(hash);

    let blob = client.blob_get(inclusion_height, namespace, commitment).await?;

    let header = client.header_get_by_height(inclusion_height).await?;

    Ok((blob, header, commitment))
}

pub fn write_header_to_file(header: &ExtendedHeader, path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(&header.header)?;
    fs::write(path, json)?;
    Ok(())
}

pub async fn verify_blob_inclusion(client: &Client, header: &ExtendedHeader, blob: &Blob) -> Result<ShareProof> {
    let eds_size = header.dah.row_roots().len() as u64;
    let ods_size = eds_size / 2;
    let first_row_index = blob.index.unwrap() / eds_size;
    let ods_index = blob.index.unwrap() - (first_row_index * ods_size);

    // NOTE: mutated the app version in the header as otherwise we run into v4 unsupported issues
    let mut modified_header = header.clone();
    modified_header.header.version.app = 3;

    let range_response = client
        .share_get_range(&modified_header, ods_index, ods_index + blob.shares_len() as u64)
        .await?;

    let share_proof = range_response.proof;
    share_proof.verify(modified_header.dah.hash())?;

    Ok(share_proof)
}

pub fn build_blob_proof_input(
    blob: &Blob,
    namespace: Namespace,
    header: &ExtendedHeader,
    share_proof: &ShareProof,
) -> KeccakInclusionToDataRootProofInput {
    let keccak_hash: [u8; 32] = Keccak256::new().chain_update(&blob.data).finalize().into();

    KeccakInclusionToDataRootProofInput {
        data: blob.data.clone(),
        namespace_id: namespace,
        share_proofs: share_proof.share_proofs.clone(),
        row_proof: share_proof.row_proof.clone(),
        data_root: header.dah.hash().as_bytes().try_into().unwrap(),
        keccak_hash,
    }
}

fn build_data_root_proof_input(header: &ExtendedHeader) -> Result<(Vec<u8>, Proof<TmSha2Hasher>), Box<dyn Error>> {
    let mut header_field_tree: MerkleTree<MemDb<[u8; 32]>, TmSha2Hasher> = MerkleTree::with_hasher(TmSha2Hasher::new());

    let field_bytes = prepare_header_fields(header);
    for leaf in field_bytes {
        header_field_tree.push_raw_leaf(&leaf);
    }

    // The data_hash is the leaf at index 6 in the tree.
    let (data_hash_bytes, data_hash_proof) = header_field_tree.get_index_with_proof(6);

    // Verify the computed root matches the header hash
    assert_eq!(header.hash().as_ref(), header_field_tree.root());

    Ok((data_hash_bytes, data_hash_proof))
}

fn prepare_header_fields(header: &ExtendedHeader) -> Vec<Vec<u8>> {
    vec![
        Protobuf::<RawConsensusVersion>::encode_vec(header.header.version),
        header.header.chain_id.clone().encode_vec(),
        header.header.height.encode_vec(),
        header.header.time.encode_vec(),
        Protobuf::<RawBlockId>::encode_vec(header.header.last_block_id.unwrap_or_default()),
        header.header.last_commit_hash.unwrap_or_default().encode_vec(),
        header.header.data_hash.unwrap_or_default().encode_vec(),
        header.header.validators_hash.encode_vec(),
        header.header.next_validators_hash.encode_vec(),
        header.header.consensus_hash.encode_vec(),
        header.header.app_hash.clone().encode_vec(),
        header.header.last_results_hash.unwrap_or_default().encode_vec(),
        header.header.evidence_hash.unwrap_or_default().encode_vec(),
        header.header.proposer_address.encode_vec(),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let provider = Provider::<Http>::try_from(config::EVM_RPC_URL)?;
    let latest_block = provider.get_block(BlockNumber::Latest).await?;

    let block_number = latest_block.unwrap().number.unwrap().as_u64();
    println!("Generating proof data for block: {:#?}", block_number);

    let genesis_path = env::var("GENESIS_PATH").expect("GENESIS_PATH must be set");
    let (genesis, chain_spec) = load_chain_spec_from_genesis(&genesis_path)?;

    generate_and_write_stf(
        config::EVM_RPC_URL,
        block_number,
        chain_spec,
        genesis,
        "testdata/client_input.bin",
    )
    .await?;

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let celestia_client = Client::new(config::CELESTIA_RPC_URL, None)
        .await
        .context("Failed creating Celestia RPC client")?;

    let (blob, header, _) =
        fetch_blob_and_header(&celestia_client, config::INDEXER_URL, block_number, namespace).await?;

    let header_json = serde_json::to_string_pretty(&header.header)?;
    fs::write("testdata/header.json", header_json)?;

    let share_proof = verify_blob_inclusion(&celestia_client, &header, &blob).await?;
    let proof_input = build_blob_proof_input(&blob, namespace, &header, &share_proof);

    let enc_blob_proof = bincode::serialize(&proof_input)?;
    fs::write("testdata/blob_proof.bin", enc_blob_proof)?;

    let (data_root, data_root_proof) = build_data_root_proof_input(&header)?;

    let hasher = TmSha2Hasher {};
    data_root_proof
        .verify_range(
            &header.header.hash().as_bytes().try_into().unwrap(),
            &[hasher.hash_leaf(&data_root)], // NOTE: that data_root has been encoded by encode_vec() which is a trait that comes from Protobuf
        )
        .expect("failed to verify header proof");

    let data_root_proof_enc = bincode::serialize(&data_root_proof)?;
    fs::write("testdata/data_root_proof.bin", data_root_proof_enc)?;

    println!("Successfully generated proof input data");

    Ok(())
}
