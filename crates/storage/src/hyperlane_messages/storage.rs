/// This module contains the implementation of the HyperlaneMessageStore, which is a wrapper around the RocksDB database.
/// It is used to store and retrieve Hyperlane messages.
/// The messages are stored in a column family called "messages".
use crate::Storage;
use anyhow::{Context, Result};
use dotenvy::dotenv;
use evm_state_types::StoredHyperlaneMessage;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct HyperlaneMessageStore {
    pub db: Arc<RwLock<DB>>,
}

impl Storage for HyperlaneMessageStore {
    fn from_path_relative(crate_depth: usize) -> Result<Self> {
        dotenv().ok();
        let mut workspace_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        for _ in 0..crate_depth {
            workspace_path = workspace_path.parent().unwrap().to_path_buf();
        }
        let relative = env::var("HYPERLANE_MESSAGE_STORE").expect("HYPERLANE_MESSAGE_STORE must be set");
        let path = workspace_path.join(relative);
        let opts = HyperlaneMessageStore::get_opts()?;
        let cfs = HyperlaneMessageStore::get_cfs()?;
        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
        })
    }

    fn get_opts() -> Result<Options> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        Ok(opts)
    }

    fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>> {
        Ok(vec![ColumnFamilyDescriptor::new("messages", Options::default())])
    }
}

impl HyperlaneMessageStore {
    pub fn insert_message(&self, index: u32, message: StoredHyperlaneMessage) -> Result<()> {
        // Serialize outside the lock to minimize lock duration
        let serialized = bincode::serialize(&message).context("Failed to serialize message")?;

        let write_lock = self
            .db
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        let cf = write_lock
            .cf_handle("messages")
            .context("Missing messages column family")?;
        write_lock
            .put_cf(cf, index.to_be_bytes(), serialized)
            .context("Failed to insert message into database")?;
        Ok(())
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
