#![allow(unused)]

pub const MAILBOX_ADDRESS: &str = "0xb1c938f5ba4b3593377f399e12175e8db0c787ff";
pub const MERKLE_TREE_ADDRESS: &str = "0xfcb1d485ef46344029d9e8a7925925e146b3430e";
// initial trusted evm height for block prover
pub const TRUSTED_HEIGHT: u64 = 166;
// celestia start height for block prover
pub const START_HEIGHT: u64 = 46;
// number of celestia blocks to prove
pub const NUM_BLOCKS: u64 = 18;
// target height for message prover
pub const TARGET_HEIGHT: u64 = 230;
// trusted evm root for block prover
pub const TRUSTED_ROOT: &str = "0xc1e771f973da5b71bc1bce5ceac9ad9d87ddf6066e326a5fb77e003b21a9d1f3";
pub const EV_RPC: &str = "http://127.0.0.1:8545";
pub const SEQUENCER_URL: &str = "http://127.0.0.1:7331";
pub const EV_WS: &str = "ws://127.0.0.1:8546";
