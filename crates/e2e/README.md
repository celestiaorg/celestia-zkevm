# ZKISM E2E on Celestia


## Manually running the E2E
In order to generate a message proof for submission to the ZK ISM, ensure that the docker environment is running:
```bash
make start
```

Run `transfer` to bridge from Celestia to EVM, wait a few seconds and run `transfer-back` to emit a hyperlane mailbox event on EVM.
```bash
make transfer
make transfer-back
```
The output of `transfer-back` will include the EVM block height at which the event was emitted. This is our `TARGET_HEIGHT` in `e2e/src/config.rs`.

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
