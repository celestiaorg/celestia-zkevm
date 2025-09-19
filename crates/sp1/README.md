## Overview

This directory contains crates for ZK programs using SP1. 

- `ev-exec` contains an SP1 program for proving EVM block execution and data availability in celestia.
- `ev-exec-types` contains types used for SP1 program IO such as the set of committed outputs.
- `ev-range-exec` contains an SP1 program which aggregates proofs output by the `ev-exec` program.
