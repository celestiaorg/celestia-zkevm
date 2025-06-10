## Overview

This directory contains crates for ZK programs using SP1. 

- `evm-exec` contains an SP1 program for proving EVM block execution and data availability in celestia
- `evm-exec-types` contains types using for SP1 program IO such as the set of commited outputs.
- `evm-range-exec` contains an SP1 program which aggregates proofs output by the `evm-exec` program.