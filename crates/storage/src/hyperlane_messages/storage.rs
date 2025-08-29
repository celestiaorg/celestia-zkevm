/// This module contains the implementation of the HyperlaneMessageStore, which is a wrapper around the RocksDB database.
/// It is used to store and retrieve Hyperlane messages.
/// The messages are stored in a column family called "messages".
/// The index of the message is the key, and the message is the value.
/// The index is a u32, and the message is a Vec<u8>.
/// The message is the raw bytes of the message, including the header and the body.
use crate::Storage;
use anyhow::{Context, Result};
use dotenvy::dotenv;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

pub struct HyperlaneMessageStore {
    pub db: Arc<DB>,
}

impl Storage for HyperlaneMessageStore {
    fn default() -> Result<Self> {
        dotenv().ok();
        let opts = HyperlaneMessageStore::get_opts()?;
        let cfs = HyperlaneMessageStore::get_cfs()?;
        let relative = "testdata/messages/hyperlane".to_string();
        let db_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(relative);
        let db = DB::open_cf_descriptors(&opts, &db_path, cfs)?;
        Ok(Self { db: Arc::new(db) })
    }

    fn from_env() -> Result<Self> {
        dotenv().ok();
        let opts = HyperlaneMessageStore::get_opts()?;
        let cfs = HyperlaneMessageStore::get_cfs()?;
        let relative = env::var("HYPERLANE_MESSAGE_STORE").unwrap_or("testdata/messages/hyperlane".to_string());
        let db_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(relative);
        let db = DB::open_cf_descriptors(&opts, &db_path, cfs)?;
        Ok(Self { db: Arc::new(db) })
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
    pub fn insert_message(&self, index: usize, message: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle("messages").context("Missing CF")?;
        self.db.put_cf(cf, index.to_be_bytes(), message)?;
        Ok(())
    }

    pub fn get_message(&self, index: usize) -> Result<Vec<u8>> {
        let cf = self.db.cf_handle("messages").context("Missing CF")?;
        let message = self.db.get_cf(cf, index.to_be_bytes())?;
        message.context("Failed to get message")
    }

    pub fn current_index(&self) -> Result<usize> {
        let cf = self.db.cf_handle("messages").context("Missing CF")?;
        let iter = self.db.iterator_cf(cf, IteratorMode::Start);
        Ok(iter.count())
    }
}
