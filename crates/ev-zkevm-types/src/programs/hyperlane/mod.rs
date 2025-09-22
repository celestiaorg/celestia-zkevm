pub mod tree;
pub mod types;
use sha3::{Digest, Keccak256};

pub fn digest_keccak(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
