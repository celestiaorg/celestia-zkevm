/// This module contains the implementation of the HyperlaneMessageStore, which is a wrapper around the RocksDB database.
/// It is used to store and retrieve Hyperlane messages.
/// The messages are stored in a column family called "messages".
use anyhow::{Context, Result};
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options, SliceTransform};
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::hyperlane::StoredHyperlaneMessage;

pub struct HyperlaneMessageStore {
    pub db: Arc<RwLock<DB>>,
}

impl HyperlaneMessageStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        db_opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(8)); // apply globally too

        let cfs = Self::get_cfs()?;
        let db = DB::open_cf_descriptors(&db_opts, path, cfs)?;
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
        })
    }

    pub fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>> {
        let mut cf_opts = Options::default();
        cf_opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(8));
        Ok(vec![ColumnFamilyDescriptor::new("messages", cf_opts)])
    }

    /// Insert a serialized hyperlane message into the database
    pub fn insert_message(&self, index: u64, message: StoredHyperlaneMessage) -> Result<()> {
        let serialized = bincode::serialize(&message)?;
        let write_lock = self.db.write().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;

        if let Some(block) = message.block_number {
            let cf_blk = write_lock.cf_handle("messages").expect("Missing messages CF");
            let mut key = block.to_be_bytes().to_vec();
            key.extend_from_slice(&index.to_be_bytes()); // 16-byte key
            write_lock.put_cf(cf_blk, key, &serialized)?;
        }

        Ok(())
    }

    /// Get all stored Hyperlane messages for a given block height
    pub fn get_by_block(&self, block: u64) -> Result<Vec<StoredHyperlaneMessage>> {
        let db = self.db.read().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;
        let cf_blk = db.cf_handle("messages").context("Missing CF")?;
        let mut result = Vec::new();
        let prefix = block.to_be_bytes();
        let iter = db.prefix_iterator_cf(cf_blk, prefix);
        for kv in iter {
            let (_k, v) = kv?;
            result.push(bincode::deserialize(&v)?);
        }
        Ok(result)
    }

    /// Get the next index to use for insertion.
    pub fn current_index(&self) -> Result<u64> {
        let db = self.db.read().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;
        let cf = db.cf_handle("messages").context("Missing messages CF")?;
        let mut iter = db.iterator_cf(cf, IteratorMode::End);
        if let Some(Ok((k, _))) = iter.next() {
            if k.len() != 16 {
                anyhow::bail!("messages CF key length != 16 (got {})", k.len());
            }
            // key = block(8) || index(8)
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&k[8..16]);
            return Ok(u64::from_be_bytes(buf) + 1);
        }
        Ok(0)
    }

    /// Prune all Hyperlane messages from the database
    pub fn prune_all(&self) -> Result<()> {
        let mut write_lock = self
            .db
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {e}"))?;
        write_lock.drop_cf("messages")?;
        let mut cf_opts = Options::default();
        cf_opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(8));
        write_lock.create_cf("messages", &cf_opts)?;
        Ok(())
    }
}
