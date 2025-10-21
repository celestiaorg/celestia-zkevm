//! Library for RISC0 guest program that verifies inclusion of EVM blocks in the Celestia data availability network
//! and executes their state transition functions using Zeth.
//!
//! ## Functionality
//!
//! The program accepts the following inputs:
//! - Celestia block header and associated data availability header (DAH).
//! - Namespace
//! - Blobs
//! - Sequencer Public Key
//! - NamespaceProofs
//! - Zeth Inputs (EVM execution witnesses)
//! - Trusted Height
//! - Trusted State Root
//!
//! It performs the following steps:
//! 1. Deserializes the program inputs.
//! 2. Verifies completeness of the namespace using the provided blobs.
//! 3. Executes the EVM blocks via Zeth's stateless validation.
//! 4. Filters blobs to SignedData and verifies the sequencer signature.
//! 5. Verifies equivalency between the EVM block data and blob data via SignedData.
//! 6. Commits a [`BlockExecOutput`] struct to the program outputs.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloy_consensus::Header;
use alloy_primitives::{B256, FixedBytes, keccak256};
use alloy_rlp;
use bytes::Bytes;
use celestia_types::{
    Blob, DataAvailabilityHeader,
    nmt::{Namespace, NamespaceProof, NamespacedHash, EMPTY_LEAVES},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use nmt_rs::NamespacedSha2Hasher;
use prost::Message as ProstMessage;
use serde::{Deserialize, Serialize};
use tendermint::block::Header as CelestiaHeader;
use zeth_core::{EthEvmConfig, Input as ZethInput, validate_block};

/// Input for RISC0 block execution circuit
#[derive(Serialize, Deserialize, Debug)]
pub struct Risc0BlockExecInput {
    pub header_raw: Vec<u8>,
    pub dah: DataAvailabilityHeader,
    pub blobs_raw: Vec<u8>,
    pub pub_key: Vec<u8>,
    pub namespace: Namespace,
    pub proofs: Vec<NamespaceProof>,
    pub zeth_inputs: Vec<ZethInput>,
    pub trusted_height: u64,
    pub trusted_root: FixedBytes<32>,
}

/// Output from block execution
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockExecOutput {
    pub celestia_header_hash: [u8; 32],
    pub prev_celestia_header_hash: [u8; 32],
    pub new_height: u64,
    pub new_state_root: [u8; 32],
    pub prev_height: u64,
    pub prev_state_root: [u8; 32],
    pub namespace: Namespace,
    pub public_key: [u8; 32],
}

/// SignedData protobuf message structure
#[derive(Clone, PartialEq, prost::Message)]
pub struct SignedData {
    #[prost(message, optional, tag = "1")]
    pub data: Option<Data>,
    #[prost(message, optional, tag = "2")]
    pub signer: Option<Signer>,
    #[prost(bytes = "vec", tag = "3")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Data {
    #[prost(message, repeated, tag = "1")]
    pub l2_headers: Vec<L2Header>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct L2Header {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Signer {
    #[prost(bytes = "vec", tag = "1")]
    pub pub_key: Vec<u8>,
}

pub fn verify_and_execute(input: Risc0BlockExecInput) -> Result<BlockExecOutput, &'static str> {
    // 1. Deserialize Celestia header and blobs
    let celestia_header: CelestiaHeader = serde_cbor::from_slice(&input.header_raw)
        .map_err(|_| "Failed to deserialize Celestia header")?;
    let blobs: Vec<Blob> = serde_cbor::from_slice(&input.blobs_raw)
        .map_err(|_| "Failed to deserialize blobs")?;

    // 2. Verify namespace data
    verify_namespace_data(&input, &celestia_header, &blobs)?;

    // 3. Execute EVM blocks using Zeth
    let headers = execute_evm_blocks(&input)?;

    // 4. Verify signed data
    let tx_data = verify_signed_data(&input, blobs, &headers)?;

    // 5. Verify blob equivalency
    verify_blob_equivalency(&headers, tx_data)?;

    // 6. Build output
    let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(input.trusted_height);
    let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(input.trusted_root);

    Ok(BlockExecOutput {
        celestia_header_hash: celestia_header.hash().as_bytes().try_into().unwrap(),
        prev_celestia_header_hash: celestia_header.last_block_id.unwrap().hash.as_bytes().try_into().unwrap(),
        new_height,
        new_state_root: new_state_root.into(),
        prev_height: input.trusted_height,
        prev_state_root: input.trusted_root.into(),
        namespace: input.namespace,
        public_key: input.pub_key.try_into().unwrap(),
    })
}

fn verify_namespace_data(
    input: &Risc0BlockExecInput,
    celestia_header: &CelestiaHeader,
    blobs: &[Blob],
) -> Result<(), &'static str> {
    // Verify DAH matches header
    if celestia_header.data_hash.unwrap() != input.dah.hash() {
        return Err("DataHash mismatch for DataAvailabilityHeader");
    }

    // Find row roots containing namespace
    let mut roots = Vec::<&NamespacedHash>::new();
    for row_root in input.dah.row_roots() {
        if row_root.contains::<NamespacedSha2Hasher<29>>(input.namespace.into()) {
            roots.push(row_root);
        }
    }

    if roots.len() != input.proofs.len() {
        return Err("Number of proofs must equal the number of row roots");
    }

    if roots.is_empty() && !blobs.is_empty() {
        return Err("Blobs must be empty if no roots contain namespace");
    }

    // Extract blob data as shares
    let blob_data: Vec<[u8; 512]> = blobs
        .iter()
        .flat_map(|blob| {
            blob.to_shares()
                .unwrap()
                .into_iter()
                .map(|share| share.as_ref().try_into().unwrap())
        })
        .collect();

    // Verify proofs
    let mut cursor = 0;
    for (proof, root) in input.proofs.iter().zip(roots) {
        if proof.is_of_absence() {
            proof
                .verify_complete_namespace(root, EMPTY_LEAVES, input.namespace.into())
                .map_err(|_| "Failed to verify absence proof")?;
            break;
        }
        let share_count = (proof.end_idx() - proof.start_idx()) as usize;
        let end = cursor + share_count;
        let raw_leaves = &blob_data[cursor..end];
        proof
            .verify_complete_namespace(root, raw_leaves, input.namespace.into())
            .map_err(|_| "Failed to verify namespace proof")?;
        cursor = end;
    }

    Ok(())
}

fn execute_evm_blocks(input: &Risc0BlockExecInput) -> Result<Vec<Header>, &'static str> {
    let mut headers = Vec::with_capacity(input.zeth_inputs.len());

    if !input.zeth_inputs.is_empty() {
        // Create EVM config for Ethereum mainnet
        let evm_config = EthEvmConfig::new((*zeth_chainspec::MAINNET).clone());

        for zeth_input in &input.zeth_inputs {
            // Execute using Zeth's stateless validation
            let _state_root = validate_block(zeth_input.clone(), evm_config.clone())
                .map_err(|_| "EVM block execution failed")?;

            // Extract header from the validated block
            headers.push(zeth_input.block.header.clone());
        }
    }

    Ok(headers)
}

fn verify_signed_data(
    input: &Risc0BlockExecInput,
    blobs: Vec<Blob>,
    headers: &[Header],
) -> Result<Vec<Data>, &'static str> {
    let signed_data: Vec<SignedData> = blobs
        .into_iter()
        .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
        .collect();

    let mut tx_data: Vec<Data> = Vec::new();
    for sd in signed_data {
        let signer = sd.signer.as_ref().ok_or("SignedData must contain signer")?;

        // Trim 4 byte Protobuf encoding prefix
        if signer.pub_key.len() < 4 || signer.pub_key[4..] != input.pub_key[..] {
            continue;
        }

        let data_bytes = sd.data.as_ref().ok_or("SignedData must contain data")?.encode_to_vec();
        let pub_key_array: [u8; 32] = input.pub_key[..].try_into().unwrap();
        let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
            .map_err(|_| "Invalid public key")?;
        let signature = Signature::from_bytes(&sd.signature.try_into().unwrap());

        verifying_key.verify(&data_bytes, &signature)
            .map_err(|_| "Signature verification failed")?;

        tx_data.push(sd.data.unwrap());
    }

    if tx_data.len() != headers.len() {
        return Err("Number of signed data must match number of headers");
    }

    Ok(tx_data)
}

fn verify_blob_equivalency(headers: &[Header], tx_data: Vec<Data>) -> Result<(), &'static str> {
    for (header, data) in headers.iter().zip(tx_data.iter()) {
        if data.l2_headers.len() != 1 {
            return Err("Data must contain exactly one L2 header");
        }

        let l2_header = &data.l2_headers[0];
        let header_hash = keccak256(alloy_rlp::encode(header));

        if header_hash.as_slice() != l2_header.hash {
            return Err("Header hash mismatch");
        }
    }

    Ok(())
}
