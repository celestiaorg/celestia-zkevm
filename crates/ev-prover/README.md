## Overview

The `ev-prover` service is a simple gRPC service designed to serve ZK proofs to clients.
It encapsulates the SP1 programs maintained under `sp1`, and uses the `sp1_sdk::ProverClient` in order to interface with them.

### Running the ev-prover service

Run the following commands from the root of the repository.

1. Install the binary to local Cargo binary directory `~/.cargo/bin`:

    ```shell
    cargo install --path ./crates/ev-prover
    ```

2. Initialise a new `ev-prover` home directory and configuration file with defaults:

    ```shell
    ev-prover init
    ```

3. Start the `ev-prover` application binary using:

    ```shell
    RUST_LOG="ev_prover=debug" ev-prover start
    ```

4. Verify the service is up and running using `grpcurl`:

    ```shell
    grpcurl -plaintext localhost:50052 list
    ```

### Build system

This crate contains a custom `build.rs` that instructs the Cargo build system how to handle compile-time build dependencies to be included by the application.

The `build.rs` contains steps to walk the `proto` directory at the root of the repository, collect all relevant files and compile them.
This outputs generated code to `src/proto` as well as a `descriptor.bin` which is leveraged by the reflection service.

TODO: include section about building sp1 programs when added
