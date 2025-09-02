use anyhow::{anyhow, Result};
use async_trait::async_trait;
use celestia_types::nmt::Namespace;
use evm_exec_types::{BlockExecOutput, BlockRangeExecOutput};
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofStorageError {
    #[error("Database error: {0}")]
    Database(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
    #[error("Proof not found for height: {0}")]
    ProofNotFound(u64),
    #[error("Range proof not found for range: {0}-{1}")]
    RangeProofNotFound(u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBlockProof {
    pub celestia_height: u64,
    pub celestia_header_hash: [u8; 32],
    pub evm_height: u64,
    pub evm_state_root: [u8; 32],
    pub namespace: Namespace,
    pub proof_data: Vec<u8>,
    pub public_values: Vec<u8>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRangeProof {
    pub id: u64,
    pub start_height: u64,
    pub end_height: u64,
    pub celestia_header_hash: [u8; 32],
    pub trusted_height: u64,
    pub trusted_state_root: [u8; 32],
    pub new_height: u64,
    pub new_state_root: [u8; 32],
    pub proof_data: Vec<u8>,
    pub public_values: Vec<u8>,
    pub created_at: u64,
}

#[async_trait]
pub trait ProofStorage: Send + Sync {
    async fn store_block_proof(
        &self,
        celestia_height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &BlockExecOutput,
    ) -> Result<(), ProofStorageError>;

    async fn store_range_proof(
        &self,
        start_height: u64,
        end_height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &BlockRangeExecOutput,
    ) -> Result<(), ProofStorageError>;

    async fn get_block_proof(&self, celestia_height: u64) -> Result<StoredBlockProof, ProofStorageError>;

    async fn get_range_proofs(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredRangeProof>, ProofStorageError>;

    async fn get_block_proofs_in_range(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredBlockProof>, ProofStorageError>;

    async fn get_latest_block_proof(&self) -> Result<Option<StoredBlockProof>, ProofStorageError>;
}

pub struct RocksDbProofStorage {
    db: Arc<DB>,
}

const CF_BLOCK_PROOFS: &str = "block_proofs";
const CF_RANGE_PROOFS: &str = "range_proofs";
const CF_METADATA: &str = "metadata";

impl RocksDbProofStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ProofStorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_BLOCK_PROOFS, Options::default()),
            ColumnFamilyDescriptor::new(CF_RANGE_PROOFS, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        Ok(Self { db: Arc::new(db) })
    }

    fn get_cf(&self, name: &str) -> Result<&ColumnFamily, ProofStorageError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| anyhow!("Column family {} not found", name).into())
    }

    fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>, ProofStorageError> {
        Ok(bincode::serialize(data)?)
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T, ProofStorageError> {
        Ok(bincode::deserialize(data)?)
    }

    fn height_key(&self, height: u64) -> [u8; 8] {
        height.to_be_bytes()
    }

    fn range_key(&self, start: u64, end: u64) -> [u8; 16] {
        let mut key = [0u8; 16];
        key[..8].copy_from_slice(&start.to_be_bytes());
        key[8..].copy_from_slice(&end.to_be_bytes());
        key
    }

    fn get_next_range_id(&self) -> Result<u64, ProofStorageError> {
        let cf = self.get_cf(CF_METADATA)?;
        let key = b"next_range_id";

        let current_id = match self.db.get_cf(cf, key)? {
            Some(bytes) => {
                let mut id_bytes = [0u8; 8];
                id_bytes.copy_from_slice(&bytes);
                u64::from_be_bytes(id_bytes)
            }
            None => 1,
        };

        let next_id = current_id + 1;
        self.db.put_cf(cf, key, &next_id.to_be_bytes())?;

        Ok(current_id)
    }
}

#[async_trait]
impl ProofStorage for RocksDbProofStorage {
    async fn store_block_proof(
        &self,
        celestia_height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &BlockExecOutput,
    ) -> Result<(), ProofStorageError> {
        let cf = self.get_cf(CF_BLOCK_PROOFS)?;

        let stored_proof = StoredBlockProof {
            celestia_height,
            celestia_header_hash: output.celestia_header_hash,
            evm_height: output.new_height,
            evm_state_root: output.new_state_root,
            namespace: output.namespace,
            proof_data: proof.bytes(),
            public_values: proof.public_values.to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let key = self.height_key(celestia_height);
        let value = self.serialize(&stored_proof)?;

        self.db.put_cf(cf, key, value)?;
        Ok(())
    }

    async fn store_range_proof(
        &self,
        start_height: u64,
        end_height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &BlockRangeExecOutput,
    ) -> Result<(), ProofStorageError> {
        let cf = self.get_cf(CF_RANGE_PROOFS)?;
        let id = self.get_next_range_id()?;

        let stored_proof = StoredRangeProof {
            id,
            start_height,
            end_height,
            celestia_header_hash: output.celestia_header_hash,
            trusted_height: output.trusted_height,
            trusted_state_root: output.trusted_state_root,
            new_height: output.new_height,
            new_state_root: output.new_state_root,
            proof_data: proof.bytes(),
            public_values: proof.public_values.to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let key = self.range_key(start_height, end_height);
        let value = self.serialize(&stored_proof)?;

        self.db.put_cf(cf, key, value)?;
        Ok(())
    }

    async fn get_block_proof(&self, celestia_height: u64) -> Result<StoredBlockProof, ProofStorageError> {
        let cf = self.get_cf(CF_BLOCK_PROOFS)?;
        let key = self.height_key(celestia_height);

        match self.db.get_cf(cf, key)? {
            Some(data) => Ok(self.deserialize(&data)?),
            None => Err(ProofStorageError::ProofNotFound(celestia_height)),
        }
    }

    async fn get_range_proofs(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredRangeProof>, ProofStorageError> {
        let cf = self.get_cf(CF_RANGE_PROOFS)?;
        let start_key = self.range_key(start_height, 0);
        let end_key = self.range_key(end_height, u64::MAX);

        let mut results = Vec::new();
        let iter = self
            .db
            .iterator_cf(cf, rocksdb::IteratorMode::From(&start_key, rocksdb::Direction::Forward));

        for item in iter {
            let (key, value) = item?;
            if key.as_ref() > end_key.as_slice() {
                break;
            }

            let proof: StoredRangeProof = self.deserialize(&value)?;
            if proof.start_height >= start_height && proof.end_height <= end_height {
                results.push(proof);
            }
        }

        Ok(results)
    }

    async fn get_block_proofs_in_range(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredBlockProof>, ProofStorageError> {
        let cf = self.get_cf(CF_BLOCK_PROOFS)?;
        let start_key = self.height_key(start_height);
        let end_key = self.height_key(end_height);

        let mut results = Vec::new();
        let iter = self
            .db
            .iterator_cf(cf, rocksdb::IteratorMode::From(&start_key, rocksdb::Direction::Forward));

        for item in iter {
            let (key, value) = item?;
            if key.as_ref() > end_key.as_slice() {
                break;
            }

            let proof: StoredBlockProof = self.deserialize(&value)?;
            results.push(proof);
        }

        Ok(results)
    }

    async fn get_latest_block_proof(&self) -> Result<Option<StoredBlockProof>, ProofStorageError> {
        let cf = self.get_cf(CF_BLOCK_PROOFS)?;

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::End);

        for item in iter {
            let (_, value) = item?;
            let proof: StoredBlockProof = self.deserialize(&value)?;
            return Ok(Some(proof));
        }

        Ok(None)
    }
}
