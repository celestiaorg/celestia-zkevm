## Overview

An SP1 program that proves the root and leaf count of the Hyperlane MerkleTreeHook incremental tree against a given `state_root`.
Individual messages are then verified on the destination chain with a merkle proof against the proven root, passed as ISM metadata:
`leaf_index (u32, big-endian) || branch (32 × 32 bytes)`.

### Program Inputs
| Name | Type | Description |
|---|---|---|
| state_root | String | The state root of the execution client reth at the target height |
| contract | Address | The address of the MerkleTreeHook contract |
| tree_proof | HyperlaneBranchProofInputs | Storage proof of the 32 tree branch nodes (slots 151-182) and the count (slot 183) |

### Program Outputs
Bincode encoded as a fixed 101 bytes. The version byte lets the destination chain reject public values produced by
any other program version, and the length can never match the legacy message id layout.

| Name | Type | Description |
|---|---|---|
| version | u8 | The output layout version, currently `1` |
| state_root | [u8; 32] | The state root of the execution client reth at the target height |
| merkle_tree_address | [u8; 32] | The MerkleTreeHook contract address, left-padded to 32 bytes |
| tree_root | [u8; 32] | The root of the incremental tree |
| count | u32 | The number of leaves in the incremental tree |


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
The binary accepts a number of flags, `contract` the contract Address of the MerkleTreeHook contract, `height` the evm block height whose state root the tree is proven against, `rpc_url` of the reth/execution client, and optionally `out_dir` to write a Groth16 `proof.bin` and `public_values.bin` when proving.

Running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `ev-hyperlane` binary in execution mode.

```shell
RUST_LOG=info cargo run -p ev-hyperlane-script --release -- --execute --contract 0x6007cE81D2FD7b9b7f22e71cE9896e00d6017ba8 --height 268 --rpc-url http://127.0.0.1:8545
```

Run the `ev-hyperlane` binary in proving mode.

```shell
RUST_LOG=info cargo run -p ev-hyperlane-script -release -- --prove ...
```
