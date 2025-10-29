## Overview

An SP1 program that verifies a sequence of N `ev-exec` proofs.
See [crates/sp1/ev-exec](../ev-exec/).

### Program Inputs
| Name | Type | Description |
|---|---|---|
| vkeys | Vec<[u8;32]> | Verifying keys for the input proofs |
| public_values | Vec<Vec<u8>> | Public outputs of the input proofs |

Note that by design proofs are written separately in recursive circuits like this one.
We use the standard SP1 approach to write compressed proofs generated using `ev-exec`.

### Program Outputs
| Name | Type | Description |
|---|---|---|
| prev_celestia_header_hash | [u8;32] | the trusted Celestia header hash in the ISM |
| prev_celestia_height | u64 | the trusted Celestia height in the ISM |
| celestia_header_hash | [u8;32] | the new Celestia header hash after applying the blocks |
| celestia_height | u64 | the new Celestia height after applying the blocks |
| trusted_height | u64 | the trusted EV height in the ISM | 
| trusted_state_root | [u8;32] | the trusted EV state root in the ISM |
| new_height | u64 | the new EV height after applying the blocks |
| new_state_root | [u8;32] | the new EV state root after applying the blocks |
| namespace | [u8;29] | the Celestia namespace that contains the data blobs which themselves contain EV blocks | |
| public_key | [u8;32] | the sequencer's public key for verification |



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

The `script` crate contains three binaries and depends on proofs generated from `ev-exec` and output to the `testdata` directory maintained at the root of the repository, thus all `cargo` commands should be run from there.

1. Run the `vkey` binary to output the verifier key for the `ev-range-exec` program.

    ```shell
    cargo run -p ev-range-exec-script --bin vkey-ev-range-exec --release
    ```

2. The `ev-range-exec` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.

Note, running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `ev-range-exec` binary in execution mode.

```shell
RUST_LOG=info cargo run -p ev-range-exec-script --release -- --execute
```

Run the `ev-range-exec` binary in proving mode.

```shell
RUST_LOG=info cargo run -p ev-range-exec-script --release -- --prove
```

3. The `parser` binary can be used to read an existing `SP1ProofWithPublicValues` from `testdata/groth16-proof.bin` and split it into its constituent components:
the Groth16 proof and the associated public inputs.

This is useful for working with external tools or runtimes which require the raw proof and public inputs as distinct artifacts.

The script expects an existing file at `testdata/groth16-proof.bin` and will output:
- `testdata/proof.bin`: the Groth16 proof bytes
- `testdata/sp1-inputs.bin`: the serialized public values

Run the `parser` binary to split the proof into the raw Groth16 proof and associated public values.

```shell
RUST_LOG=info cargo run -p ev-range-exec-script --bin parser --release
```

Please refer to https://docs.succinct.xyz/docs/sp1/introduction for more comprehensive documentation on Succinct SP1.
