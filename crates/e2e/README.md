# ZKISM E2E on Celestia

## 1. Message Proof Payload
In order to generate a message proof for submission to the ZK ISM, ensure that the docker environment is running:
```bash
make start
```

Submit several transactions by running:
```bash
make transfer
make transfer-back-loop
```
Take note of one of the block heights after transaction submission, so that you can update the `target_height` in the config file.

Next update the configuration file in `e2e/config.rs`:

In `e2e/config.rs`:
```rust
...
pub const TARGET_HEIGHT: u64 = 250;
```
Make sure that before TARGET_HEIGHT transactions have occurred. The E2E will prove all transactions in range `0` to `TARGET_HEIGHT` in one go.


Next, run the prover:
```bash
cargo run -p e2e --bin message-prover --release
```

## 2. Block Proof Payload
In order to generate a block range proof for submission to the ZK ISM,
make sure that the configuration file in `e2e/src/config.rs` contains the correct checkpoint and target values:

```rust
pub const MAILBOX_ADDRESS: &str = "0xb1c938f5ba4b3593377f399e12175e8db0c787ff";
pub const MERKLE_TREE_ADDRESS: &str = "0xfcb1d485ef46344029d9e8a7925925e146b3430e";
// initial trusted height for block prover
pub const TRUSTED_HEIGHT: u64 = 0;
// height at wich to start proving blocks
pub const START_HEIGHT: u64 = 2;
// number of blocks to prove for block prover (from TRUSTED_HEIGHT onwards)
pub const NUM_BLOCKS: u64 = 2;
// target height for message prover
pub const TARGET_HEIGHT: u64 = 100;
pub const TRUSTED_ROOT: &str = "0x2892acb3938e55f74887eb9624668f2c5f0d97fae9151d83dea3b70d5ea850b5";
pub const EV_RPC: &str = "http://127.0.0.1:8545";
pub const EV_WS: &str = "ws://127.0.0.1:8546";
```

## Config Breakdown
## ZK ISM
`TRUSTED_HEIGHT`: The EVM trusted height from the ZK ISM.
`TRUSTED_ROOT`: The trusted EVM root from the ZK ISM

## Block Prover
`START_HEIGHT`: The Celestia block height of the block after the one that included the last trusted EVM block.
NUM_BLOCKS: The number of Celestia blocks whose EVM blocks we want to prove, starting at START_HEIGHT.

## Message Prover
`TARGET_HEIGHT`: The EVM target height for our message prover, should be set to the last EVM height of the Celestia block whose height is START_HEIGHT + NUM_BLOCKS. (the last evm height of the last celestia block that we prove with the block prover).

For our E2E this will tell the message prover to prove all messages from block 0 to the last EVM block in our target Celestia block.

Next, run the prover:
```bash
cargo run -p e2e --bin block-prover --release
```


## Wiring up the E2E
The E2E test should call both the `prove_blocks` method from `prover/blocks.rs` and the `prove_messages` method from `prover/message.rs`. They will return a single `groth16` proof each that can then be sent to the ZK ISM on Celestia.

The trusted state must match exactly the trusted state on Celestia and the message proof must be for the new `TRUSTED_ROOT` and `TRUSTED_HEIGHT` that is in the output of the groth16 range block proof.