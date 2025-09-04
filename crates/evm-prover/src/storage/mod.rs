pub mod proof_storage;
#[cfg(test)]
mod tests;

pub use proof_storage::{ProofStorage, RocksDbProofStorage};
