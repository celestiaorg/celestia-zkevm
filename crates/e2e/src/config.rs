#![allow(unused)]
/// E2E config
pub mod e2e {
    pub const ISM_ID: &str = "0x726f757465725f69736d000000000000000000000000002a0000000000000001";
    // target height for message prover
    pub const TARGET_HEIGHT: u64 = 207;
    pub const SEQUENCER_URL: &str = "http://127.0.0.1:7331";
}

/// Other configs (block, message binaries)
pub mod other {
    pub const MAILBOX_ADDRESS: &str = "0xb1c938f5ba4b3593377f399e12175e8db0c787ff";
    pub const MERKLE_TREE_ADDRESS: &str = "0xfcb1d485ef46344029d9e8a7925925e146b3430e";
    // initial trusted evm height for block prover
    pub const TRUSTED_HEIGHT: u64 = 165;
    // target height for message prover
    pub const TARGET_HEIGHT: u64 = 198;
    // trusted evm root for block prover
    pub const TRUSTED_ROOT: &str = "0x0e106db5b2dd79354e2ae0116439ee1fa4fcf88bdec03803c9c79bf0e1101f08";
    pub const EV_RPC: &str = "http://127.0.0.1:8545";
    pub const SEQUENCER_URL: &str = "http://127.0.0.1:7331";
    pub const EV_WS: &str = "ws://127.0.0.1:8546";
}
