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