## Overview

An SP1 program that verifies the existence of Hyperlane Messages against a given `state_root`.

### Program Inputs
| Name | Type | Description |
|---|---|---|
| state_root | String | The state root of the execution client reth at the target height |
| contract | Address | The address of the MerkleTreeHook contract |
| messages | Vec<HyperlaneMessage> | The messages that are stored locally, pass only the DB path when using CLI|
| branch proof | EIP1186AccountProofResponse | Storage proof object for verifying the on-chain Tree branch |
| snapshot | MerkleTree | The snapshot of the Merkle Tree after previous inserts, e.g. the starting point for this proof |

### Program Outputs
| Name | Type | Description |
|---|---|---|
| state_root | String | The state root of the execution client reth at the target height for verification |
| messages | Vec<String> | The ids of the Hyperlane messages that we proofed |


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

1. Run the `vkey` binary to output the verifier key for the `ev-hyperlane` program.

    ```shell
    cargo run -p ev-hyperlane-script --bin vkey-ev-hyperlane --release
    ```

2. The `ev-hyperlane` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.
The binary accepts a number of flags, `contract` the contract Address of the MerkleTreeHook contract, `start_height` the height of the block that contains the first message in our local db for this proof, `target_height` the target evm block height containing the last message that we are trying to generate a proof for, `rpc_url` of the reth/execution client.

Running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `ev-hyperlane` binary in execution mode.

```shell
RUST_LOG=info cargo run -p ev-hyperlane-script --release -- --execute --contract 0xFCb1d485ef46344029D9E8A7925925e146B3430E --start-height 0 --target-height 268 --rpc-url http://127.0.0.1:8545
```

Run the `ev-hyperlane` binary in proving mode.

```shell
RUST_LOG=info cargo run -p ev-hyperlane-script -release -- --prove ...
```