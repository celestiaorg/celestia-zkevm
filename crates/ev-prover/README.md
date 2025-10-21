## Overview

The `ev-prover` service is a simple gRPC service designed to serve ZK proofs to clients.
It supports multiple proof system backends:
- **SP1** (default): Uses the SP1 programs maintained under `crates/sp1` via the `sp1_sdk::ProverClient`
- **RISC0**: Uses the RISC0 programs maintained under `crates/risc0` via the `risc0_zkvm` library

Both proof systems share the same core verification logic from the `ev-zkevm-types` crate, ensuring consistency.

### Building with Features

The proof systems are controlled by Cargo features:
- `sp1` (default): Includes SP1 support
- `risc0`: Includes RISC0 support

Build with specific features:
```bash
# SP1 only (default)
cargo build --release

# RISC0 only
cargo build --release --no-default-features --features risc0

# Both SP1 and RISC0
cargo build --release --features risc0
```

The active proof system can be selected via the `PROOF_SYSTEM` environment variable or in the configuration file.

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
    # Using SP1 (default)
    RUST_LOG="ev_prover=debug" ev-prover start

    # Or explicitly with SP1
    RUST_LOG="ev_prover=debug" PROOF_SYSTEM=sp1 ev-prover start

    # Using RISC0
    RUST_LOG="ev_prover=debug" PROOF_SYSTEM=risc0 ev-prover start
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
