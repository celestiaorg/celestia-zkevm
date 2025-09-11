#![allow(dead_code)]
use std::sync::RwLock;

use alloy_primitives::FixedBytes;
use sp1_sdk::include_elf;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

pub struct AppContext {
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub trusted_state: RwLock<TrustedState>,
}

pub struct TrustedState {
    // the index of the snapshot that we will load from the db, initially 0 (empty by default)
    snapshot_index: u64,
    // the index of the last message we proofed successfully, initially 0
    message_index: u64,
    // the block height of the state_root on chain
    height_on_chain: u64,
    // the state_root on chain that we will verify against
    root_on_chain: FixedBytes<32>,
}

impl TrustedState {
    pub fn new(snapshot_index: u64, message_index: u64, height_on_chain: u64, root_on_chain: FixedBytes<32>) -> Self {
        Self {
            snapshot_index,
            message_index,
            height_on_chain,
            root_on_chain,
        }
    }
}

pub struct HyperlaneMessageProver {
    pub app: AppContext,
}
