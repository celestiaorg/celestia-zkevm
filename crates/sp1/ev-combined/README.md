## Overview

### Program Inputs
| Name | Type | Description |
|---|---|---|
| blocks | Vec<BlockExecInput> | Inputs required to execute a range of blocks |


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

These are the same as for `ev-range-exec`, because the circuits fulfill a similar purpose and update the same ISM endpoint. The only difference is that `combined` does not prove every single block recursively, but instead takes a block range as input and generates a single proof using just one prover instance.