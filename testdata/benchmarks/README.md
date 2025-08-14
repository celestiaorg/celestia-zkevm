## Benchmarking

See [`BENCHMARKS.md`](./BENCHMARKS.md) for results.

The following benchmarks have been created using the `script` crates included in each of the sp1 programs `evm-exec` and `evm-range-exec`.
Please refer to the `README.md` files in under [sp1/evm-exec](../../crates/sp1/evm-exec/README.md) and [sp1/evm-range-exec](../../crates/sp1/evm-range-exec/README.md) for instructions on how to invoke the SP1 programs.

### Collecting proof input data

The docker compose network can be used in order to collect proof input data. 
Note, each document in this repository assumes that commands are run from the root of the repository.

Launch the docker compose services:

```shell
make start
```

Stop the docker compose services:

```shell
make stop
```

Collect proof input data for a range of blocks.
As mentioned in the `evm-exec` documentation, using the `--start` and `--blocks` flags allows the user to chose a starting height and number of blocks to gather input data for.

```shell
cargo run -p evm-exec-script --bin data-gen --release -- --start 15 --blocks 5
```

### Benchmarking the block execution program

Once proof input data has been collected. The user can invoke the SP1 block execution program `evm-exec` and output a benchmark report to a JSON file.
Start by invoking the program for block-15:

```shell
RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 15 --output-file block-15.json
```

Using the outputs displayed on screen we can invoke the program for the subsequent blocks passing the `--trusted-height` and `--trusted-root` flags.
Note, as we are chaining celestia blocks sequentially, we do not have a guarantee that each block will contain a `SignedData` blob for the EVM application.
Thus, we explitcitly pass these flags to ensure empty blocks maintain a trusted height and state root moving forward.

```shell
RUST_LOG=info cargo run -p evm-exec-script --release -- --execute --height 16 --output-file block-15.json --trusted-height {N} --trusted-root {abcdef...}
```

### Benchmarking the block range/aggregation program

In order to run the block range/aggregation program `evm-range-exec`, we must use real proofs. We can generate real proofs using the SP1 prover network by invoking the same commands as we did above, passing the `--prove` flag in place of `--execute`.

Running the block execution program in `--prove` mode will write the proof output containing both the SP1 compressed proof and public values to the `testdata/proofs` directory.

The block range program can then be invoked which will automatically take all of the proofs in `testdata/proofs` and attempt to verify them in sequence.

```shell
RUST_LOG=info cargo run -p evm-range-exec-script --release -- --execute --output-file blocks-15-19.json
```

### Generating benchmark charts

The benchmark reports will contain raw data included in JSON files under `testdata/benchmarks`.
We can generate charts for these JSON data files easily by running a simple python script.

Note, the python script requires `python3` and `matplotlib`.

Run the `benchcharts.py` script to generate benchmark reports in a visual format.

```shell
python3 testdata/benchcharts.py
```
