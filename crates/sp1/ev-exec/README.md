## Overview

An SP1 program that verifies inclusion of EVM reth blocks in the Celestia data availability network 
and executes their state transition functions.


### Program Inputs
| Name | Type | Description |
|---|---|---|
| header_raw | Vec<u8> | the Celestia block header |
| dah | DataAvailabilityHeader | the Celestia DA header |
| blobs_raw | Vec<u8> | The bytes of the data blobs |
| pub_key | Vec<u8> | The sequencer pubkey for verification |
| namespace | Namespace | the Celestia namespace that contains the data blobs which themselves contain EV blocks |
| proofs | Vec<NamespaceProof> | Merkle Proofs for the Namespace data, can be exclusion |
| executor_inputs | Vec<EthClientExecutorInput> | Struct that aggregates the RETH inputs for block execution |
| trusted_height | u64 | Trusted EV height from the previous block |
| trusted_root | [u8;32] | Trusted EV root from the previous block |


Note that by design proofs are written separately in recursive circuits like this one.
We use the standard SP1 approach to write compressed proofs generated using `ev-exec`.

### Program Outputs
| Name | Type | Description |
|---|---|---|
| celestia_header_hash | [u8;32] | the new Celestia header hash after applying the blocks |
| prev_celestia_height | u64 | the trusted Celestia height in the ISM |
| prev_celestia_header_hash | [u8;32] | the trusted Celestia header hash in the ISM |
| new_height | u64 | the new EV height after applying the blocks |
| new_state_root | [u8;32] | the new EV state root after applying the blocks |
| prev_height | u64 | the height of the previous EV block |
| prev_state_root | [u8;32] | the EV state root of the previous block |
| namespace | [u8;29] | the Celestia namespace that contains the data blobs which themselves contain EV blocks | |
| public_key | [u8;32] | the sequencer's public key for verification |


### Equivocation Tolerance

A Byzantine or malicious sequencer node may **equivocate**—that is, submit multiple `SignedData` payloads for the same block height to the Celestia data availability network.  

The proof system cannot prevent this behavior, but it can be designed to **tolerate** it. Celestia not only provides data availability for applications, it also enforces an **ordering mechanism** (see the [Data Square Layout specification](https://celestiaorg.github.io/celestia-app/data_square_layout.html#ordering)).  

In the event of equivocation, the proof system accepts the payload according to the first-come, first-served (FCFS) rule enforced by Celestia’s priority-based ordering. This ensures that proof generation remains deterministic and avoids divergent execution paths that could otherwise arise without a clear fork-choice policy.

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
cargo run -p ev-exec-script --bin data-gen --release -- --start <START_BLOCK> --blocks <END_BLOCK>
```

2. Run the `vkey` binary to output the verifier key for the `ev-exec` program.

    ```shell
    cargo run -p ev-exec-script --bin vkey-ev-exec --release
    ```

3. The `ev-exec` binary can be run in both `--execute` and `--prove` mode. Execution mode will run the program without generating a proof.
Proving mode will attempt to generate a proof for the program which can be verified using the programs verification key and public inputs.
The binary accepts a number of flags, `--height` the Celestia block height, `--trusted-height` the trusted EVM height and `--trusted-root` 
the trusted state root for the trusted height. Please note, the `--trusted-height` and `--trusted-root` flags are required when proving an 
empty Celestia block (i.e. a Celestia block containing no tx data for the EVM application).

Running the program in proving mode requires the `SP1_PROVER` and optionally the `NETWORK_PRIVATE_KEY` env variables to be set.
See `.env.example` at the root of the repository.

Run the `ev-exec` binary in execution mode.

```shell
RUST_LOG=info cargo run -p ev-exec-script --release -- --execute --height 12 --trusted-height 18 --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
```

Run the `ev-exec` binary in proving mode.

```shell
RUST_LOG=info cargo run -p ev-exec-script --release -- --prove --height 12 --trusted-height 18 --trusted-root c02a6bbc8529cbe508a24ce2961776b699eeb6412c99c2e106bbd7ebddd4d385
```

4. When running the program in `--execute` mode, the user can also optionally provide the `--output-file` flag.
For example:
```shell
RUST_LOG=info cargo run -p ev-exec-script --release -- --execute --height 10 --output-file output.json
```

This will write a `BenchmarkReport` JSON object containing the results of the program execution to: `testdata/benchmarks/output.json`.
This includes total gas, total instruction count, total syscall count as well as a breakdown of cycle trackers used within the program.

Please refer to https://docs.succinct.xyz/docs/sp1/introduction for more comprehensive documentation on Succinct SP1.
