/// This module contains the implementation of the HyperlaneMessageStore, which is a wrapper around the RocksDB database.
/// It is used to store and retrieve Hyperlane messages.
/// The messages are stored in a column family called "messages".
use anyhow::{Context, Result};
use dotenvy::dotenv;
use evm_state_types::StoredHyperlaneMessage;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub enum IndexMode {
    Block,
    Message,
}

pub struct HyperlaneMessageStore {
    pub db: Arc<RwLock<DB>>,
    pub index_mode: IndexMode,
}

impl HyperlaneMessageStore {
    pub fn from_path_relative(crate_depth: usize, index_mode: IndexMode) -> Result<Self> {
        dotenv().ok();
        let mut workspace_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        for _ in 0..crate_depth {
            workspace_path = workspace_path.parent().unwrap().to_path_buf();
        }
        let relative = env::var("HYPERLANE_MESSAGE_STORE").expect("HYPERLANE_MESSAGE_STORE must be set");
        let path = workspace_path.join(relative);
        let opts = Self::get_opts()?;
        let cfs = Self::get_cfs()?;
        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
            index_mode,
        })
    }

    pub fn get_opts() -> Result<Options> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        Ok(opts)
    }

    pub fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>> {
        Ok(vec![
            ColumnFamilyDescriptor::new("messages", Options::default()),
            ColumnFamilyDescriptor::new("messages_by_block", Options::default()),
        ])
    }

    pub fn insert_message(&self, index: u32, message: StoredHyperlaneMessage) -> Result<()> {
        let serialized = bincode::serialize(&message)?;

        let write_lock = self.db.write().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let cf_msg = write_lock.cf_handle("messages").context("Missing messages CF")?;

        match self.index_mode {
            IndexMode::Block => {
                if let Some(block) = message.block_number {
                    let cf_blk = write_lock
                        .cf_handle("messages_by_block")
                        .context("Missing by_block CF")?;
                    // allow multiple per block: key = (block, index)
                    let mut key = block.to_be_bytes().to_vec();
                    key.extend_from_slice(&index.to_be_bytes());
                    write_lock.put_cf(cf_blk, key, &serialized)?;
                }
            }
            IndexMode::Message => {
                write_lock.put_cf(cf_msg, index.to_be_bytes(), &serialized)?;
            }
        }

        Ok(())
    }

    pub fn get_by_block(&self, block: u64) -> Result<Vec<StoredHyperlaneMessage>> {
        let read_lock = self.db.read().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let cf = read_lock.cf_handle("message_by_block").context("Missing CF")?;

        let mut result = Vec::new();
        let prefix = block.to_be_bytes();

        // Iterate only keys that start with `block`
        let iter = read_lock.prefix_iterator_cf(cf, prefix);
        for kv in iter {
            let (_k, v) = kv?;
            result.push(bincode::deserialize(&v)?);
        }
        Ok(result)
    }

    pub fn get_message(&self, index: u32) -> Result<StoredHyperlaneMessage> {
        let read_lock = self
            .db
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
        let cf = read_lock.cf_handle("messages").context("Missing CF")?;
        let message = read_lock
            .get_cf(cf, index.to_be_bytes())?
            .context("Failed to get message")?;
        bincode::deserialize(&message).context("Failed to deserialize message")
    }

    pub fn current_index(&self) -> Result<u32> {
        let read_lock = self
            .db
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
        let cf = read_lock.cf_handle("messages").context("Missing CF")?;
        let mut iter = read_lock.iterator_cf(cf, IteratorMode::End);
        if let Some(Ok((k, _))) = iter.next() {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&k); // safe since key is always 4 bytes
            Ok(u32::from_be_bytes(buf) + 1)
        } else {
            Ok(0)
        }
    }

    pub fn prune_all(&self) -> Result<()> {
        let mut write_lock = self
            .db
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        write_lock.drop_cf("messages")?;
        let opts = Options::default();
        write_lock.create_cf("messages", &opts)?;
        Ok(())
    }
}
