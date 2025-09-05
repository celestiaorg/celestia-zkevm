use anyhow::Result;
use rocksdb::{ColumnFamilyDescriptor, Options};

use crate::hyperlane_messages::storage::HyperlaneMessageStore;

pub mod hyperlane_messages;

// every storage module should implement this trait
pub trait Storage {
    fn from_env() -> Result<HyperlaneMessageStore>;
    fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>>;
    fn get_opts() -> Result<Options>;
}
