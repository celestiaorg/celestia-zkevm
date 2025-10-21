use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

use alloy_consensus::{proofs, BlockHeader};
use alloy_primitives::{B256, FixedBytes};
use alloy_rlp::Decodable;
use bytes::Bytes;
use celestia_types::{
    Blob, DataAvailabilityHeader,
    nmt::{Namespace, NamespaceProof, NamespacedHash, EMPTY_LEAVES},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hex::encode;
use nmt_rs::NamespacedSha2Hasher;
use prost::Message as ProstMessage;
use reth_primitives::TransactionSigned;
use rsp_client_executor::{executor::EthClientExecutor, io::{EthClientExecutorInput, WitnessInput}};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tendermint::block::Header;

/// BlockExecInput is the input for the BlockExec circuit.
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockExecInput {
    pub header_raw: Vec<u8>,
    pub dah: DataAvailabilityHeader,
    pub blobs_raw: Vec<u8>,
    pub pub_key: Vec<u8>,
    pub namespace: Namespace,
    pub proofs: Vec<NamespaceProof>,
    pub executor_inputs: Vec<EthClientExecutorInput>,
    pub trusted_height: u64,
    pub trusted_root: FixedBytes<32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockExecOutput {
    // celestia_header_hash is the merkle hash of the Celestia block header.
    pub celestia_header_hash: [u8; 32],
    // prev_celestia_header_hash is the merkle hash of the previous Celestia block header.
    pub prev_celestia_header_hash: [u8; 32],
    // new_height is the block number after the state transition function has been applied.
    pub new_height: u64,
    // new_state_root is the EVM application state root after the state transition function has been applied.
    pub new_state_root: [u8; 32],
    // prev_height is the block number before the state transition function has been applied.
    pub prev_height: u64,
    // prev_state_root is the EVM application state root before the state transition function has been applied.
    pub prev_state_root: [u8; 32],
    // namespace is the Celestia namespace that contains the blob data.
    pub namespace: Namespace,
    // public_key is the sequencer's public key used to verify the signatures of the signed data.
    pub public_key: [u8; 32],
}

/// Display trait implementation to format hashes as hex encoded output.
impl Display for BlockExecOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "BlockExecOutput {{")?;
        writeln!(f, "  celestia_header_hash: {}", encode(self.celestia_header_hash))?;
        writeln!(
            f,
            "  prev_celestia_header_hash: {}",
            encode(self.prev_celestia_header_hash)
        )?;
        writeln!(f, "  new_height: {}", self.new_height)?;
        writeln!(f, "  new_state_root: {}", encode(self.new_state_root))?;
        writeln!(f, "  prev_height: {}", self.prev_height)?;
        writeln!(f, "  prev_state_root: {}", encode(self.prev_state_root))?;
        writeln!(f, "  namespace: {}", encode(self.namespace.0))?;
        writeln!(f, "  public_key: {}", encode(self.public_key))?;
        writeln!(f, "}}")
    }
}

/// BlockRangeExecInput is the input for the BlockRangeExec circuit.
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockRangeExecInput {
    pub vkeys: Vec<[u32; 8]>,
    pub public_values: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockRangeExecOutput {
    // celestia_header_hash is the hash of the celestia header at which new_height is available.
    pub celestia_header_hash: [u8; 32],
    // trusted_height is the trusted height of the EVM application.
    pub trusted_height: u64,
    // trusted_state_root is the state commitment root of the EVM application at trusted_height.
    pub trusted_state_root: [u8; 32],
    // new_height is the EVM application block number after N state transitions.
    pub new_height: u64,
    // new_state_root is the computed state root of the EVM application after
    // executing N blocks from trusted_height to new_height.
    pub new_state_root: [u8; 32],
    // namespace is the Celestia namespace that contains the blob data.
    pub namespace: [u8; 29],
    // public_key is the sequencer's public key used to verify the signatures of the signed data.
    pub public_key: [u8; 32],
}

/// Display trait implementation to format hashes as hex encoded output.
impl Display for BlockRangeExecOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "BlockRangeExecOutput {{")?;
        writeln!(f, "  celestia_header_hash: {}", encode(self.celestia_header_hash))?;
        writeln!(f, "  trusted_height: {}", self.trusted_height)?;
        writeln!(f, "  trusted_state_root: {}", encode(self.trusted_state_root))?;
        writeln!(f, "  new_height: {}", self.new_height)?;
        writeln!(f, "  new_state_root: {}", encode(self.new_state_root))?;
        writeln!(f, "  namespace: {}", encode(self.namespace))?;
        writeln!(f, "  public_key: {}", encode(self.public_key))?;
        writeln!(f, "}}")
    }
}

/// A buffer of serializable/deserializable objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buffer {
    pub data: Vec<u8>,
    #[serde(skip)]
    pub ptr: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            ptr: 0,
        }
    }

    pub fn from(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            ptr: 0,
        }
    }

    #[allow(dead_code)]
    /// Set the position ptr to the beginning of the buffer.
    pub fn head(&mut self) {
        self.ptr = 0;
    }

    /// Read the serializable object from the buffer.
    pub fn read<T: Serialize + DeserializeOwned>(&mut self) -> T {
        let result: T = bincode::deserialize(&self.data[self.ptr..]).expect("failed to deserialize");
        let nb_bytes = bincode::serialized_size(&result).expect("failed to get serialized size");
        self.ptr += nb_bytes as usize;
        result
    }

    #[allow(dead_code)]
    pub fn read_slice(&mut self, slice: &mut [u8]) {
        slice.copy_from_slice(&self.data[self.ptr..self.ptr + slice.len()]);
        self.ptr += slice.len();
    }

    #[allow(dead_code)]
    /// Write the serializable object from the buffer.
    pub fn write<T: Serialize>(&mut self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("serialization failed");
        self.data.extend(tmp);
    }

    #[allow(dead_code)]
    /// Write the slice of bytes to the buffer.
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.data.extend_from_slice(slice);
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

// Import the Data and SignedData types from ev-types for blob verification
// Note: This will need to be added to Cargo.toml dependencies
use ev_types::v1::{Data, SignedData};

/// Implementation of block execution verification logic.
/// This contains all the core logic factored out from the SP1 program.
impl BlockExecInput {
    /// Verify and execute the block inputs, returning the output.
    /// This method contains all the verification logic and can be called from any zkVM.
    pub fn verify_and_execute(self) -> Result<BlockExecOutput, Box<dyn Error>> {
        // -----------------------------
        // 1. Deserialize inputs
        // -----------------------------
        let celestia_header: Header = serde_cbor::from_slice(&self.header_raw)?;
        let blobs: Vec<Blob> = serde_cbor::from_slice(&self.blobs_raw)?;

        // -----------------------------
        // 2. Verify namespace inclusion and completeness
        // -----------------------------
        self.verify_namespace_data(&celestia_header, &blobs)?;

        // -----------------------------
        // 3. Execute the EVM block inputs
        // -----------------------------
        let headers = self.execute_evm_blocks()?;

        // -----------------------------
        // 4. Filter SignedData blobs and verify signatures
        // -----------------------------
        let tx_data = self.verify_signed_data(blobs, &headers)?;

        // -----------------------------
        // 5. Verify blob equivalency
        // -----------------------------
        self.verify_blob_equivalency(&headers, tx_data)?;

        // -----------------------------
        // 6. Build and commit outputs
        // -----------------------------
        let new_height: u64 = headers.last().map(|h| h.number).unwrap_or(self.trusted_height);
        let new_state_root: B256 = headers.last().map(|h| h.state_root).unwrap_or(self.trusted_root);

        let output = BlockExecOutput {
            celestia_header_hash: celestia_header
                .hash()
                .as_bytes()
                .try_into()
                .expect("celestia_header_hash must be exactly 32 bytes"),
            prev_celestia_header_hash: celestia_header
                .last_block_id
                .unwrap()
                .hash
                .as_bytes()
                .try_into()
                .expect("prev_celestia_header_hash must be exactly 32 bytes"),
            new_height,
            new_state_root: new_state_root.into(),
            prev_height: self.trusted_height,
            prev_state_root: self.trusted_root.into(),
            namespace: self.namespace,
            public_key: self.pub_key.try_into().expect("public key must be exactly 32 bytes"),
        };

        Ok(output)
    }

    /// Verify namespace inclusion and completeness
    fn verify_namespace_data(&self, celestia_header: &Header, blobs: &[Blob]) -> Result<(), Box<dyn Error>> {
        assert_eq!(
            celestia_header.data_hash.unwrap(),
            self.dah.hash(),
            "DataHash mismatch for DataAvailabilityHeader"
        );

        let mut roots = Vec::<&NamespacedHash>::new();
        for row_root in self.dah.row_roots() {
            if row_root.contains::<NamespacedSha2Hasher<29>>(self.namespace.into()) {
                roots.push(row_root);
            }
        }

        assert_eq!(
            roots.len(),
            self.proofs.len(),
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
        for (proof, root) in self.proofs.iter().zip(roots) {
            if proof.is_of_absence() {
                proof
                    .verify_complete_namespace(root, EMPTY_LEAVES, self.namespace.into())
                    .expect("Failed to verify proof");
                break;
            }
            let share_count = (proof.end_idx() - proof.start_idx()) as usize;
            let end = cursor + share_count;

            let raw_leaves = &blob_data[cursor..end];

            proof
                .verify_complete_namespace(root, raw_leaves, self.namespace.into())
                .expect("Failed to verify proof");

            cursor = end;
        }

        Ok(())
    }

    /// Execute EVM blocks and return the resulting headers
    fn execute_evm_blocks(&self) -> Result<Vec<alloy_consensus::Header>, Box<dyn Error>> {
        let mut headers = Vec::with_capacity(self.executor_inputs.len());
        if headers.capacity() != 0 {
            let first_input = self.executor_inputs.first().unwrap();

            assert_eq!(
                self.trusted_root,
                first_input.state_anchor(),
                "State anchor must be equal to trusted root"
            );

            assert!(
                self.trusted_height <= first_input.parent_header().number(),
                "Trusted height must be less than or equal to parent header height",
            );

            let executor = EthClientExecutor::eth(
                Arc::new((&first_input.genesis).try_into().expect("invalid genesis block")),
                first_input.custom_beneficiary,
            );

            for input in &self.executor_inputs {
                let header = executor.execute(input.clone()).expect("EVM block execution failed");
                headers.push(header);
            }
        }

        Ok(headers)
    }

    /// Filter SignedData blobs and verify signatures
    fn verify_signed_data(&self, blobs: Vec<Blob>, headers: &[alloy_consensus::Header]) -> Result<Vec<Data>, Box<dyn Error>> {
        let signed_data: Vec<SignedData> = blobs
            .into_iter()
            .filter_map(|blob| SignedData::decode(Bytes::from(blob.data)).ok())
            .collect();

        let mut tx_data: Vec<Data> = Vec::new();
        for sd in signed_data {
            let signer = sd.signer.as_ref().expect("SignedData must contain signer");

            // NOTE: Trim 4 byte Protobuf encoding prefix
            if signer.pub_key[4..] != self.pub_key {
                continue;
            }

            let data_bytes = sd.data.as_ref().expect("SignedData must contain data").encode_to_vec();

            verify_signature(&self.pub_key, &data_bytes, &sd.signature)?;

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

        Ok(tx_data)
    }

    /// Verify blob equivalency between headers and transaction data
    fn verify_blob_equivalency(&self, headers: &[alloy_consensus::Header], tx_data: Vec<Data>) -> Result<(), Box<dyn Error>> {
        for (header, data) in headers.iter().zip(tx_data) {
            let mut txs = Vec::with_capacity(data.txs.len());
            for tx_bytes in data.txs {
                let tx = TransactionSigned::decode(&mut tx_bytes.as_slice())?;
                txs.push(tx);
            }

            let root = proofs::calculate_transaction_root(&txs);
            assert_eq!(
                root, header.transactions_root,
                "Calculated root must be equal to header transactions root"
            );
        }

        Ok(())
    }
}

fn get_height(data: &Data) -> Option<u64> {
    data.metadata.as_ref().map(|m| m.height)
}

fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Box<dyn Error>> {
    let pub_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| "Public key must be 32 bytes for Ed25519")?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key)
        .map_err(|e| format!("Invalid Ed25519 public key: {e}"))?;

    let signature = Signature::from_slice(signature)
        .map_err(|e| format!("Invalid Ed25519 signature: {e}"))?;

    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("Signature verification failed: {e}"))?;

    Ok(())
}
