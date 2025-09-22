use std::fmt::{Display, Formatter, Result as FmtResult};

use alloy_primitives::FixedBytes;
use celestia_types::{
    DataAvailabilityHeader,
    nmt::{Namespace, NamespaceProof},
};
use hex::encode;
use rsp_client_executor::io::EthClientExecutorInput;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

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
