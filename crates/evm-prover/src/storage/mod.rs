pub mod storage;
#[cfg(test)]
mod tests;

pub use storage::{ProofStorage, RocksDbProofStorage};
