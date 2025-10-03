#![allow(unused)]

// This config is an E2E artifact and still used when run the block or message binary
// The e2e binary however is fully automated and does not require any manual intervention
// besides setting the TARGET_HEIGHT, which is to be removed soon (this comment will be updated)
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
