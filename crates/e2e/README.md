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

Wait for all containers to initialize fully.

Then set your `SP1_PROVER` in the workspace `.env` to `cpu`, `cuda` or `network` and run:
```bash
RUST_LOG="e2e=info" make e2e
```

If you want to see all the details about block proving and other background processes, run:

```bash
RUST_LOG="e2e=debug" make e2e
```

## Configuration
All default parameters for interacting with Hyperlane contracts are located in `e2e/src/config.rs`. The private key used for signing Celestia messages is derived from the environment:

`.env`:
```bash
...
CELESTIA_PRIVATE_KEY="6e30efb1d3ebd30d1ba08c8d5fc9b190e08394009dc1dd787a69e60c33288a8c"
```