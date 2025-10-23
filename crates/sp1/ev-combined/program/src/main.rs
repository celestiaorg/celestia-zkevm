//! An SP1 program that verifies a sequence of N `ev-exec` proofs.
//!
//! It accepts:
//! - N verification keys
//! - N serialized public values (each from a `EvmBlockExecOutput`)
//!
//! It performs:
//! 1. Proof verification for each input
//! 2. Sequential header verification (i.e., block continuity)
//! 3. Aggregation of metadata into a `EvmRangeExecOutput`
//!
//! It commits:
//! - The trusted block height and state root
//! - The new block height and state root
//! - The latest Celestia header hash from the sequence

#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_consensus::proofs;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use celestia_types::{
    nmt::{NamespacedHash, EMPTY_LEAVES},
    Blob,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use ev_types::v1::{Data, SignedData};
use ev_zkevm_types::programs::block::{BlockExecInput, BlockExecOutput, BlockRangeExecOutput, EvCombinedInput};
use nmt_rs::NamespacedSha2Hasher;
use prost::Message;
use reth_primitives::TransactionSigned;
use rsp_client_executor::executor::EthClientExecutor;
use std::{collections::HashSet, error::Error, sync::Arc};
use tendermint::block::Header;

pub fn main() {
    let inputs: EvCombinedInput = sp1_zkvm::io::read::<EvCombinedInput>();

    let mut outputs: Vec<BlockExecOutput> = Vec::new();
    for block in inputs.blocks {
        outputs.push(verify_block(&block).expect("failed to verify block"));
    }

    for window in outputs.windows(2).enumerate() {
        let (i, pair) = window;
        let (prev, curr) = (&pair[0], &pair[1]);
        assert_eq!(
            curr.prev_height,
            prev.new_height,
            "verify sequential EVM headers failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.new_height,
            curr.prev_height
        );

        assert_eq!(
            curr.prev_state_root,
            prev.new_state_root,
            "verify sequential EVM state roots failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.new_state_root,
            curr.prev_state_root
        );

        assert_eq!(
            curr.prev_celestia_header_hash,
            prev.celestia_header_hash,
            "verify sequential Celestia headers failed at index {}: expected {:?}, got {:?}",
            i + 1,
            prev.celestia_header_hash,
            curr.prev_celestia_header_hash
        );

        assert_eq!(
            curr.namespace, prev.namespace,
            "unexpected namespace: expected {:?}, got {:?}",
            prev.namespace, curr.namespace
        );

        assert_eq!(
            curr.public_key, prev.public_key,
            "unexpected public key: expected {:?}, got {:?}",
            prev.public_key, curr.public_key
        );
    }

    let first = outputs.first().expect("No outputs provided");
    let last = outputs.last().expect("No outputs provided");

    let output = BlockRangeExecOutput {
        prev_celestia_height: first.prev_celestia_height,
        prev_celestia_header_hash: first.prev_celestia_header_hash,
        celestia_height: first.prev_celestia_height + outputs.len() as u64,
        celestia_header_hash: last.celestia_header_hash,
        trusted_height: first.prev_height,
        trusted_state_root: first.prev_state_root,
        new_state_root: last.new_state_root,
        new_height: last.new_height,
        namespace: last
            .namespace
            .as_bytes()
            .try_into()
            .expect("namespace must be 29 bytes"),
        public_key: last.public_key,
    };

    println!("cycle-tracker-report-end: build and commit outputs");

    sp1_zkvm::io::commit(&output);
}

fn verify_block(inputs: &BlockExecInput) -> Result<BlockExecOutput, Box<dyn Error>> {
    let celestia_header: Header =
        serde_cbor::from_slice(&inputs.header_raw).expect("failed to deserialize celestia header");
    let blobs: Vec<Blob> = serde_cbor::from_slice(&inputs.blobs_raw).expect("failed to deserialize blob data");

    println!("cycle-tracker-report-end: deserialize inputs");

    // -----------------------------
    // 1. Verify namespace inclusion and completeness
    // -----------------------------
    println!("cycle-tracker-report-start: verify namespace data");

    assert_eq!(
        celestia_header.data_hash.unwrap(),
        inputs.dah.hash(),
        "DataHash mismatch for DataAvailabilityHeader"
    );

    let mut roots = Vec::<&NamespacedHash>::new();
    for row_root in inputs.dah.row_roots() {
        if row_root.contains::<NamespacedSha2Hasher<29>>(inputs.namespace.into()) {
            roots.push(row_root);
        }
    }

    assert_eq!(
        roots.len(),
        inputs.proofs.len(),
        "Number of proofs must equal the number of row roots"
    );

    if roots.is_empty() {
        assert!(blobs.is_empty(), "Blobs must be empty if no roots contain namespace");
    }

    let blob_data: Vec<[u8; 512]> = blobs
        .iter()
        .flat_map(|blob| {
            blob.to_shares()
                .unwrap()
                .into_iter()
                .map(|share| share.as_ref().try_into().unwrap())
        })
        .collect();

    let mut cursor = 0;
    for (proof, root) in inputs.proofs.iter().zip(roots) {
        if proof.is_of_absence() {
            proof
                .verify_complete_namespace(root, EMPTY_LEAVES, inputs.namespace.into())
                .expect("Failed to verify proof");
            break;
        }
        let share_count = (proof.end_idx() - proof.start_idx()) as usize;
        let end = cursor + share_count;

        let raw_leaves = &blob_data[cursor..end];

        proof
            .verify_complete_namespace(root, raw_leaves, inputs.namespace.into())
            .expect("Failed to verify proof");

        cursor = end;
    }

    let mut headers = Vec::with_capacity(inputs.executor_inputs.len());
    if headers.capacity() != 0 {
        let first_input = inputs.executor_inputs.first().unwrap();

        let executor = EthClientExecutor::eth(
            Arc::new((&first_input.genesis).try_into().expect("invalid genesis block")),
            first_input.custom_beneficiary,
        );

        for input in &inputs.executor_inputs {
            let header = executor.execute(input.clone()).expect("EVM block execution failed");
            headers.push(header);
        }
    }

    let signed_data: Vec<SignedData> = blobs
        .into_iter()
        .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
        .collect();

    let mut tx_data: Vec<Data> = Vec::new();
    for sd in signed_data {
        let signer = sd.signer.as_ref().expect("SignedData must contain signer");

        // NOTE: Trim 4 byte Protobuf encoding prefix
        if signer.pub_key[4..] != inputs.pub_key {
            continue;
        }

        let data_bytes = sd.data.as_ref().expect("SignedData must contain data").encode_to_vec();

        verify_signature(&inputs.pub_key, &data_bytes, &sd.signature).expect("Sequencer signature verification failed");

        tx_data.push(sd.data.unwrap());
    }

    // Equivocation tolerance: Filter out duplicate heights if applicable, accepting FCFS as the source of truth.
    if tx_data.len() != headers.len() {
        let mut seen = HashSet::<u64>::new();
        tx_data.retain(|data| get_height(data).map(|h| seen.insert(h)).unwrap_or(false));
    }

    tx_data.sort_by_key(|data| get_height(data).expect("Data must contain a height"));

    assert_eq!(
        tx_data.len(),
        headers.len(),
        "Headers and SignedData must be of equal length"
    );

    for (header, data) in headers.iter().zip(tx_data) {
        let mut txs = Vec::with_capacity(data.txs.len());
        for tx_bytes in data.txs {
            let tx = TransactionSigned::decode(&mut tx_bytes.as_slice()).expect("Failed decoding transaction");
            txs.push(tx);
        }

        let root = proofs::calculate_transaction_root(&txs);
        assert_eq!(
            root, header.transactions_root,
            "Calculated root must be equal to header transactions root"
        );
    }

    let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(inputs.trusted_height);
    let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(inputs.trusted_root);

    let output = BlockExecOutput {
        celestia_header_hash: celestia_header
            .hash()
            .as_bytes()
            .try_into()
            .expect("celestia_header_hash must be exactly 32 bytes"),
        prev_celestia_height: celestia_header.height.value() - 1,
        prev_celestia_header_hash: celestia_header
            .last_block_id
            .unwrap()
            .hash
            .as_bytes()
            .try_into()
            .expect("prev_celestia_header_hash must be exactly 32 bytes"),
        new_height,
        new_state_root: new_state_root.into(),
        prev_height: inputs.trusted_height,
        prev_state_root: inputs.trusted_root.into(),
        namespace: inputs.namespace,
        public_key: inputs
            .pub_key
            .clone()
            .try_into()
            .expect("public key must be exactly 32 bytes"),
    };
    Ok(output)
}

fn get_height(data: &Data) -> Option<u64> {
    data.metadata.as_ref().map(|m| m.height)
}

fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Box<dyn Error>> {
    let pub_key: [u8; 32] = public_key
        .try_into()
        .map_err(|e| format!("Public key must be 32 bytes for Ed25519: {e}"))?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key).map_err(|e| format!("Invalid Ed25519 public key: {e}"))?;

    let signature = Signature::from_slice(signature).map_err(|e| format!("Invalid Ed25519 signature: {e}"))?;

    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("Signature verification failed: {e}"))?;
    Ok(())
}
