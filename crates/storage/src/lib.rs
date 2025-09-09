use anyhow::Result;
use rocksdb::{ColumnFamilyDescriptor, Options};

use crate::hyperlane_messages::storage::HyperlaneMessageStore;

pub mod hyperlane_messages;

// every storage module should implement this trait
pub trait Storage {
    fn from_path_relative(crate_depth: usize) -> Result<HyperlaneMessageStore>;
    fn get_cfs() -> Result<Vec<ColumnFamilyDescriptor>>;
    fn get_opts() -> Result<Options>;
}
