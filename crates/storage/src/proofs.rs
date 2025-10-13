use anyhow::{Result, anyhow};
use async_trait::async_trait;
use ev_zkevm_types::programs::{
    block::{BlockExecOutput, BlockRangeExecOutput},
    hyperlane::types::HyperlaneMessageOutputs,
};
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DB, Options};
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
    #[allow(dead_code)]
    RangeProofNotFound(u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBlockProof {
    pub celestia_height: u64,
    pub proof_data: Vec<u8>,
    pub public_values: Vec<u8>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRangeProof {
    pub start_height: u64,
    pub end_height: u64,
    pub proof_data: Vec<u8>,
    pub public_values: Vec<u8>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMembershipProof {
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

    #[allow(dead_code)]
    async fn store_range_proof(
        &self,
        start_height: u64,
        end_height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &BlockRangeExecOutput,
    ) -> Result<(), ProofStorageError>;

    #[allow(dead_code)]
    async fn get_block_proof(&self, celestia_height: u64) -> Result<StoredBlockProof, ProofStorageError>;

    #[allow(dead_code)]
    async fn get_range_proofs(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredRangeProof>, ProofStorageError>;

    #[allow(dead_code)]
    async fn get_block_proofs_in_range(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<StoredBlockProof>, ProofStorageError>;

    #[allow(dead_code)]
    async fn store_membership_proof(
        &self,
        height: u64,
        proof: &SP1ProofWithPublicValues,
        output: &HyperlaneMessageOutputs,
    ) -> Result<(), ProofStorageError>;

    #[allow(dead_code)]
    async fn get_membership_proof(&self, height: u64) -> Result<StoredMembershipProof, ProofStorageError>;

    #[allow(dead_code)]
    async fn get_latest_membership_proof(&self) -> Result<Option<StoredMembershipProof>, ProofStorageError>;

    #[allow(dead_code)]
    async fn get_latest_block_proof(&self) -> Result<Option<StoredBlockProof>, ProofStorageError>;
}

pub struct RocksDbProofStorage {
    db: Arc<DB>,
}

const CF_BLOCK_PROOFS: &str = "block_proofs";
const CF_RANGE_PROOFS: &str = "range_proofs";
const CF_MEMBERSHIP_PROOFS: &str = "membership_proofs";
const CF_METADATA: &str = "metadata";

impl RocksDbProofStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ProofStorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_BLOCK_PROOFS, Options::default()),
            ColumnFamilyDescriptor::new(CF_RANGE_PROOFS, Options::default()),
            ColumnFamilyDescriptor::new(CF_MEMBERSHIP_PROOFS, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        Ok(Self { db: Arc::new(db) })
    }

    fn get_cf(&self, name: &str) -> Result<&ColumnFamily, ProofStorageError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| anyhow!("Column family {name} not found").into())
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
}

#[async_trait]
impl ProofStorage for RocksDbProofStorage {
    async fn store_block_proof(
        &self,
        celestia_height: u64,
        proof: &SP1ProofWithPublicValues,
        _output: &BlockExecOutput,
    ) -> Result<(), ProofStorageError> {
        let cf = self.get_cf(CF_BLOCK_PROOFS)?;

        let stored_proof = StoredBlockProof {
            celestia_height,
            proof_data: bincode::serialize(&proof.proof)?,
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
        _output: &BlockRangeExecOutput,
    ) -> Result<(), ProofStorageError> {
        let cf = self.get_cf(CF_RANGE_PROOFS)?;

        let stored_proof = StoredRangeProof {
            start_height,
            end_height,
            proof_data: bincode::serialize(&proof.proof)?,
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

    async fn store_membership_proof(
        &self,
        height: u64,
        proof: &SP1ProofWithPublicValues,
        _output: &HyperlaneMessageOutputs,
    ) -> Result<(), ProofStorageError> {
        let cf = self.get_cf(CF_MEMBERSHIP_PROOFS)?;

        let stored_proof = StoredMembershipProof {
            proof_data: bincode::serialize(&proof.proof)?,
            public_values: proof.public_values.to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let key = self.height_key(height);
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

        let mut iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::End);

        if let Some(Ok((_, value))) = iter.next() {
            let proof: StoredBlockProof = self.deserialize(&value)?;
            return Ok(Some(proof));
        }

        Ok(None)
    }

    async fn get_membership_proof(&self, height: u64) -> Result<StoredMembershipProof, ProofStorageError> {
        let cf = self.get_cf(CF_MEMBERSHIP_PROOFS)?;
        let key = self.height_key(height);

        match self.db.get_cf(cf, key)? {
            Some(data) => Ok(self.deserialize(&data)?),
            None => Err(ProofStorageError::ProofNotFound(height)),
        }
    }

    async fn get_latest_membership_proof(&self) -> Result<Option<StoredMembershipProof>, ProofStorageError> {
        let cf = self.get_cf(CF_MEMBERSHIP_PROOFS)?;
        let mut iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::End);
        if let Some(Ok((_, value))) = iter.next() {
            let proof: StoredMembershipProof = self.deserialize(&value)?;
            return Ok(Some(proof));
        }
        Ok(None)
    }
}
