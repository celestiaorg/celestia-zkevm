## Overview

An SP1 program that verifies inclusion of an EVM reth block in the Celestia data availability network and executes its state transition function (STF).

1. Accepts an EVM block STF and associated Celestia proofs.
2. Verifies that the EVM block was included in the Celestia block.
3. Executes the EVM block STF.
4. Commits the resulting EVM and Celestia block metadata as public outputs.

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

The `script` crate contains three binaries and depends on the `testdata` directory maintained at the root of the repository, thus all `cargo` commands should be run from there.

1. Run the `data-gen` binary to scrape proof input data from services running locally.

Note, this assumes the `docker-compose` services maintained at the root of the repository are running.

    ```shell
    cargo run -p evm-exec-script --bin data-gen --release -- --start <START_BLOCK> --end <END_BLOCK>
    ```

2. Run the `vkey` binary to output the verifier key for the `evm-exec` program.

    ```shell
    cargo run -p evm-exec-script --bin vkey --release
    ```

3. The `evm-exec` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.

Note, running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `evm-exec` binary in execution mode.

    ```shell
    RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 1011
    ```

Run the `evm-exec` binary in proving mode.

    ```shell
    RUST_LOG=info cargo run -p evm-exec-script --release -- --prove --height 1011
    ```

Please refer to https://docs.succinct.xyz/docs/sp1/introduction for more comprehensive documentation on Succinct SP1.
