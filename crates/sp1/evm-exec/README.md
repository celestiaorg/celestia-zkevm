## Overview

Details about the program...

## Usage

This program comes equipped with a `script` crate for convenience.

The `script` crate uses a custom build script which employs the `sp1-build` system in order to compile the program.

```shell
cd script

cargo build
```

Run the `vkey` binary to output the verifier key for the sp1 program.

```shell
cargo run --bin vkey
```