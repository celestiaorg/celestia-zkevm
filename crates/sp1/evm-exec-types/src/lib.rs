use serde::{de::DeserializeOwned, Deserialize, Serialize};

mod hex_bytes;

#[derive(Serialize, Deserialize, Debug)]
pub struct EvmBlockExecOutput {
    // blob_commitment is the blob commitment for the EVM block.
    #[serde(with = "hex_bytes")]
    pub blob_commitment: [u8; 32],

    // header_hash is the hash of the EVM block header.
    #[serde(with = "hex_bytes")]
    pub header_hash: [u8; 32],

    // prev_header_hash is the hash of the previous EVM block header.
    #[serde(with = "hex_bytes")]
    pub prev_header_hash: [u8; 32],

    // celestia_header_hash is the merkle hash of the Celestia block header.
    #[serde(with = "hex_bytes")]
    pub celestia_header_hash: [u8; 32],

    // prev_celestia_header_hash is the merkle hash of the previous Celestia block header.
    #[serde(with = "hex_bytes")]
    pub prev_celestia_header_hash: [u8; 32],

    // new_height is the block number after the state transition function has been applied.
    pub new_height: u64,

    // new_state_root is the EVM application state root after the state transition function has been applied.
    #[serde(with = "hex_bytes")]
    pub new_state_root: [u8; 32],

    // prev_height is the block number before the state transition function has been applied.
    pub prev_height: u64,

    // prev_state_root is the EVM application state root before the state transition function has been applied.
    #[serde(with = "hex_bytes")]
    pub prev_state_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvmRangeExecOutput {
    // celestia_header_hash is the hash of the celestia header at which new_height is available.
    #[serde(with = "hex_bytes")]
    pub celestia_header_hash: [u8; 32],

    // trusted_height is the trusted height of the EVM application.
    pub trusted_height: u64,

    // trusted_state_root is the state commitment root of the EVM application at trusted_height.
    #[serde(with = "hex_bytes")]
    pub trusted_state_root: [u8; 32],

    // new_height is the EVM application block number after N state transitions.
    pub new_height: u64,

    // new_state_root is the computed state root of the EVM application after
    // executing N blocks from trusted_height to new_height.
    #[serde(with = "hex_bytes")]
    pub new_state_root: [u8; 32],
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
