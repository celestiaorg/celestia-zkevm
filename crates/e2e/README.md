# ZKISM E2E on Celestia


## Running the E2E

First 
1. Install the binary to local Cargo binary directory `~/.cargo/bin`:

    ```shell
    cargo install --path ./crates/ev-prover
    ```

2. Initialise a new `ev-prover` home directory and configuration file with defaults:

    ```shell
    ev-prover init
    ```

The newly created config will be used by the e2e binary and includes things like the EV genesis block.

In order to generate a message proof for submission to the ZK ISM, ensure that the docker environment is running:
```bash
make start
```

Run `make transfer` to bridge from Celestia to EVM, wait a few seconds and run `make transfer-back` to emit a hyperlane mailbox event on EVM.
```bash
make transfer
make transfer-back
```
The output of `make transfer-back` will include the EVM block height at which the event was emitted. This is our `TARGET_HEIGHT` in `e2e/src/config.rs`.

Find the ZKISM trusted state on Celestia:
```bash
docker exec -it celestia-validator /bin/bash
celestia-appd query zkism isms
```
The output will contain the `trusted height`.

Use curl to find the corresponding `trusted root`:
```bash
curl -s -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  --data '{
    "jsonrpc":"2.0",
    "method":"eth_getBlockByNumber",
    "params":["REPLACE_WITH_HEIGHT_AS_HEX", false],
    "id":1
  }' | jq -r '.result.stateRoot'

```

Update `e2e/src/config.rs` TRUSTED_ROOT and TRUSTED_HEIGHT to match the ones in the ZKISM.

**The corresponding inclusion heights on Celestia are derived from the TRUSTED and TARGET EVM heights in the config.**

Now set your `SP1_PROVER` in the workspace `.env` to `cpu`, `cuda` or `network` and run:
```bash
cargo run --bin e2e -p e2e --release
```