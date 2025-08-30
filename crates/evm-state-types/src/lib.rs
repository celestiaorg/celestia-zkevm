use serde::{Deserialize, Serialize};

pub mod events;
pub mod hyperlane;
use hyperlane::HyperlaneMessage;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredHyperlaneMessage {
    pub block_number: Option<u64>,
    pub message: HyperlaneMessage,
}

impl StoredHyperlaneMessage {
    pub fn new(message: HyperlaneMessage, block_number: Option<u64>) -> Self {
        Self { block_number, message }
    }
}
