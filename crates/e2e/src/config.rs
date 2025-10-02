#![allow(unused)]

pub const MAILBOX_ADDRESS: &str = "0xb1c938f5ba4b3593377f399e12175e8db0c787ff";
pub const MERKLE_TREE_ADDRESS: &str = "0xfcb1d485ef46344029d9e8a7925925e146b3430e";
// initial trusted evm height for block prover
pub const TRUSTED_HEIGHT: u64 = 0;
// celestia start height for block prover
pub const START_HEIGHT: u64 = 2;
// number of celestia blocks to prove
pub const NUM_BLOCKS: u64 = 15;
// target height for message prover
pub const TARGET_HEIGHT: u64 = 2;
// trusted evm root for block prover
pub const TRUSTED_ROOT: &str = "0x2892acb3938e55f74887eb9624668f2c5f0d97fae9151d83dea3b70d5ea850b5";
pub const EV_RPC: &str = "http://127.0.0.1:8545";
pub const EV_WS: &str = "ws://127.0.0.1:8546";
