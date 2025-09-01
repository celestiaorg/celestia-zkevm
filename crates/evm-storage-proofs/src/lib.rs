use alloy_primitives::Keccak256;

pub mod client;
pub mod types;

pub fn digest_keccak(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
