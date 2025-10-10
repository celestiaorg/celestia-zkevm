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
make e2e
```