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

This crate contains a custom `build.rs` that builds the SP1 programs used for proof generation.

### Protobuf

Protobuf is used as the canonical encoding format for gRPC messaging. The Protobuf definitions for the prover service are included in this crate under the `proto` directory.

The `buf` toolchain is employed to handle Rust code generation.
Please refer to the [official installation documentation](https://buf.build/docs/cli/installation/) to get setup with the `buf` CLI.

Rust code-gen is produced from the Protobuf definitions via `buf.gen.yaml` plugins and included in this crate under `src/proto`.

#### Regenerating Protobuf code

When making changes to the Protobuf definitions in `proto/prover/v1/prover.proto`, regenerate the Rust code by running:

```bash
cd crates/ev-prover/proto
buf generate
```

This will generate the prost message types and tonic server/client stubs compatible with prost 0.12 and tonic 0.10.

#### Protobuf development

To update the Protobuf dependencies:

```bash
cd crates/ev-prover/proto
buf dep update
```

To lint the Protobuf definitions:

```bash
cd crates/ev-prover/proto
buf lint
```
