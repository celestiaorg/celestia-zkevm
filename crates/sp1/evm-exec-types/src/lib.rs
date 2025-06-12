use serde::{de::DeserializeOwned, Deserialize, Serialize};

// TODO: Remove this type when happy to do so!
#[derive(Serialize, Deserialize, Debug)]
pub struct BlEvmBlockExecOutput {
    pub blob_commitment: [u8; 32], // confirm needed?
    pub header_hash: [u8; 32],
    pub prev_header_hash: [u8; 32],
    pub height: u64,
    pub gas_used: u64,         // confirm needed?
    pub beneficiary: [u8; 20], // confirm needed?
    pub state_root: [u8; 32],
    pub celestia_header_hash: [u8; 32],
    pub trusted_state_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvmBlockExecOutput {
    pub blob_commitment: [u8; 32],
    pub header_hash: [u8; 32],
    pub prev_header_hash: [u8; 32],
    pub celestia_header_hash: [u8; 32],
    pub prev_celestia_header_hash: [u8; 32],
    pub new_height: u64,
    pub new_state_root: [u8; 32],
    pub prev_height: u64,
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
        let result: T =
            bincode::deserialize(&self.data[self.ptr..]).expect("failed to deserialize");
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
