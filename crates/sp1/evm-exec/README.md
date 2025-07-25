## Overview

An SP1 program that verifies inclusion of EVM reth blocks in the Celestia data availability network 
and executes their state transition functions.

### Program Inputs

- `CelestiaHeader`: A celestia block header at height H.
- `DAH`: The associated data availability header at height H.
- `Namespace`: The namespace containing blob data.
- `PublicKey`: The public key of the sequencer signing blob data.
- `Blobs`: All blobs in the namespace for the current block.
- `NamespaceProofs`: Namespaced Merkle Tree proofs for the complete namespace.
- `EthClientExecutorInputs`: List of RSP based EVM state transition functions for N blocks included at height H.
- `TrustedHeight`: A trusted height containing the trusted state root.
- `TrustedStateRoot`: A trusted state root for the trusted height.

### Program Outputs

- `CelestiaHeaderHash`: A hash of the celestia block header at height H.
- `PreviousCelestiaHeaderHash`: A hash of the previous celestia block header at height H-1.
- `NewHeight`: The height of the EVM application after applying N blocks.
- `NewStateRoot`: The state root of the EVM application after applying N blocks.
- `TrustedHeight`: The trusted height of the EVM application before applying N blocks.
- `TrustedStateRoot`: The trusted state root of the EVM application before applying N blocks.
- `Namespace`: The namespace containing blob data.
- `PublicKey`: The public key of the sequencer signing blob data.

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
cargo run -p evm-exec-script --bin data-gen --release -- --start <START_BLOCK> --blocks <END_BLOCK>
```

2. Run the `vkey` binary to output the verifier key for the `evm-exec` program.

    ```shell
    cargo run -p evm-exec-script --bin vkey-evm-exec --release
    ```

3. The `evm-exec` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.
The binary accepts a number of flags, `--height` the Celestia block height, `--trusted-height` the trusted EVM height and `--trusted-root` 
the trusted state root for the trusted height. Please note, the `--trusted-height` and `--trusted-root` flags are required when proving an 
empty Celestia block (i.e. a Celestia block containing no tx data for the EVM application).

Running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `evm-exec` binary in execution mode.

```shell
RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 12 --trusted-height 18 --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
```

Run the `evm-exec` binary in proving mode.

```shell
RUST_LOG=info cargo run -p evm-exec-script --release -- --prove --height 12 --trusted-height 18 --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
```

4. When running the program in `--execute` mode, the user can also optionally provide the `--output-file` flag.
For example:
```shell
RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 10 --output-file output.json
```

This will write a `BenchmarkReport` JSON object containing the results of the program execution to: `testdata/benchmarks/output.json`.
This includes total gas, total instruction count, total syscall count as well as a breakdown of cycle trackers used within the program.

Please refer to https://docs.succinct.xyz/docs/sp1/introduction for more comprehensive documentation on Succinct SP1.
