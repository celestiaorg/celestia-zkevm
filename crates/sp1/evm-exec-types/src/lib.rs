mod hex_bytes;

use hex::encode_upper;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

#[derive(Serialize, Deserialize, Debug)]
pub struct EvmBlockExecOutput {
    #[serde(with = "hex_bytes")]
    pub blob_commitment: [u8; 32],

    #[serde(with = "hex_bytes")]
    pub header_hash: [u8; 32],

    #[serde(with = "hex_bytes")]
    pub prev_header_hash: [u8; 32],

    #[serde(with = "hex_bytes")]
    pub celestia_header_hash: [u8; 32],

    #[serde(with = "hex_bytes")]
    pub prev_celestia_header_hash: [u8; 32],

    pub new_height: u64,

    #[serde(with = "hex_bytes")]
    pub new_state_root: [u8; 32],

    pub prev_height: u64,

    #[serde(with = "hex_bytes")]
    pub prev_state_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvmRangeExecOutput {
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
}

impl Display for EvmBlockExecOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "EvmBlockExecOutput {{")?;
        writeln!(f, "  blob_commitment: {}", encode_upper(self.blob_commitment))?;
        writeln!(f, "  header_hash: {}", encode_upper(self.header_hash))?;
        writeln!(f, "  prev_header_hash: {}", encode_upper(self.prev_header_hash))?;
        writeln!(f, "  celestia_header_hash: {}", encode_upper(self.celestia_header_hash))?;
        writeln!(
            f,
            "  prev_celestia_header_hash: {}",
            encode_upper(self.prev_celestia_header_hash)
        )?;
        writeln!(f, "  new_height:                {}", self.new_height)?;
        writeln!(f, "  new_state_root:            {}", encode_upper(self.new_state_root))?;
        writeln!(f, "  prev_height:               {}", self.prev_height)?;
        writeln!(f, "  prev_state_root:           {}", encode_upper(self.prev_state_root))?;
        write!(f, "}}")
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
