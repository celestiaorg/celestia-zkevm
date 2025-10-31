## Overview

This directory contains crates for ZK programs using SP1. 

- `ev-batch-exec` SP1 program to prove a batch of blocks with minimal overhead from spawning instances.
- `ev-exec` contains an SP1 program for proving EVM block execution and data availability in celestia.
- `ev-hyperlane` SP1 program to prove hyperlane messages against an incremental tree root.
- `ev-range-exec` contains an SP1 program which aggregates proofs output by the `ev-exec` program.
