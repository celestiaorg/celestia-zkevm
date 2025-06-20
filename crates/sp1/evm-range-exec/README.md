## Overview

An SP1 program that verifies a sequence of N `evm-exec` proofs.
See [crates/sp1/evm-exec](../evm-exec/).

It accepts:
- N verification keys (TODO: The verification key)
- N serialized public values (each from a `EvmBlockExecOutput`)

It performs:
1. Proof verification for each input.
2. Sequential header verification (i.e., block continuity).
3. Aggregation of metadata into a `EvmRangeExecOutput`.

It commits:
- The trusted block height and state root
- The new block height and state root
- The latest Celestia header hash from the sequence

## Usage

The SP1 program can be compiled and used within any application binary by providing a custom `build.rs` which employs the `sp1-build` system:

```rust
use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../program", Default::default());
}
```

The compiled ELF can then be included within the application binary using the `include_elf!` macro and setup using the `ProverClient` from the `sp1-sdk`. 

## Script 

This program contains a `script` crate for convenience and to demonstrate how the program is used.

The `script` crate contains two binaries and depends proofs generated from `evm-exec` and output to the `testdata` directory maintained at the root of the repository, thus all `cargo` commands should be run from there.

1. Run the `vkey` binary to output the verifier key for the `evm-range-exec` program.

    ```shell
    cargo run -p evm-range-exec-script --bin vkey --release
    ```

2. The `evm-range-exec` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.

Note, running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `evm-range-exec` binary in execution mode.

    ```shell
    RUST_LOG=info cargo run -p evm-range-exec-script --release -- --execute
    ```

Run the `evm-range-exec` binary in proving mode.

    ```shell
    RUST_LOG=info cargo run -p evm-range-exec-script --release -- --prove
    ```

Please refer to https://docs.succinct.xyz/docs/sp1/introduction for more comprehensive documentation on Succinct SP1.
