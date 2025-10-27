// This module contains the HyperlaneShapshotStore, which is a wrapper around the RocksDB database.
// It is used to store and retrieve Hyperlane snapshots.
// The snapshots are stored in a column family called "snapshots".

use anyhow::{Context, Result};
use ev_zkevm_types::programs::hyperlane::tree::{MerkleTree, ZERO_BYTES};
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HyperlaneSnapshot {
    pub height: u64,
    pub tree: MerkleTree,
}
impl HyperlaneSnapshot {
    pub fn new(height: u64, tree: MerkleTree) -> HyperlaneSnapshot {
        HyperlaneSnapshot { height, tree }
    }
}

pub struct HyperlaneSnapshotStore {
    pub db: Arc<RwLock<DB>>,
}

impl HyperlaneSnapshotStore {
    pub fn new<P: AsRef<Path>>(path: P, trusted_snapshot: Option<MerkleTree>) -> Result<Self> {
        let opts = Self::get_opts()?;
        let cfs = Self::get_cfs()?;
        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        let snapshot_store = Self {
            db: Arc::new(RwLock::new(db)),
        };
        if let Some(trusted_snapshot) = trusted_snapshot {
            snapshot_store
                .insert_snapshot(0, HyperlaneSnapshot::new(0, trusted_snapshot))
                .context("Failed to insert trusted snapshot")?;
        } else {
            snapshot_store.insert_snapshot(0, HyperlaneSnapshot::new(0, MerkleTree::default()))?;
        }
        Ok(snapshot_store)
    }

    pub fn get_opts() -> Result<Options> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        Ok(opts)
    }

    pub fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>> {
        Ok(vec![ColumnFamilyDescriptor::new("snapshots", Options::default())])
    }

    pub fn insert_snapshot(&self, index: u64, snapshot: HyperlaneSnapshot) -> Result<()> {
        // Serialize outside the lock to minimize lock duration
        let serialized = bincode::serialize(&snapshot).context("Failed to serialize snapshot")?;

        let write_lock = self
            .db
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {e}"))?;
        let cf = write_lock
            .cf_handle("snapshots")
            .context("Missing snapshots column family")?;
        write_lock
            .put_cf(cf, index.to_be_bytes(), serialized)
            .context("Failed to insert snapshot into database")?;
        Ok(())
    }

    pub fn get_snapshot(&self, index: u64) -> Result<HyperlaneSnapshot> {
        let read_lock = self.db.read().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;
        let cf = read_lock.cf_handle("snapshots").context("Missing CF")?;
        let snapshot_bytes = read_lock
            .get_cf(cf, index.to_be_bytes())?
            .context("Failed to get snapshot")?;
        let mut snapshot: HyperlaneSnapshot = bincode::deserialize(&snapshot_bytes)?;

        // normalize: replace "" with ZERO_BYTES
        for h in snapshot.tree.branch.iter_mut() {
            if h.is_empty() {
                *h = ZERO_BYTES.to_string();
            }
        }

        Ok(snapshot)
    }

    pub fn current_index(&self) -> Result<u64> {
        let read_lock = self
            .db
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {e}"))?;
        let cf = read_lock.cf_handle("snapshots").context("Missing CF")?;
        let mut iter = read_lock.iterator_cf(cf, IteratorMode::End);
        if let Some(Ok((k, _))) = iter.next() {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&k);
            Ok(u64::from_be_bytes(buf))
        } else {
            Ok(0)
        }
    }

    pub fn reset_db(&self) -> Result<()> {
        let mut write_lock = self
            .db
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {e}"))?;
        write_lock.drop_cf("snapshots")?;
        let opts = Options::default();
        write_lock.create_cf("snapshots", &opts)?;
        Ok(())
    }
}
