# Input Type Conversion for RISC0

## Problem

The SP1 and RISC0 backends use different EVM execution engines:
- **SP1**: Uses RSP (Reth State Prover) with `EthClientExecutorInput`
- **RISC0**: Uses Zeth with `zeth_core::Input`

This means the input types are fundamentally different and cannot be directly converted.

## Current Solution

The RISC0 guest program defines its own input type `Risc0BlockExecInput` which accepts Zeth-format execution witnesses.

```rust
pub struct Risc0BlockExecInput {
    pub header_raw: Vec<u8>,
    pub dah: DataAvailabilityHeader,
    pub blobs_raw: Vec<u8>,
    pub pub_key: Vec<u8>,
    pub namespace: Namespace,
    pub proofs: Vec<NamespaceProof>,
    pub zeth_inputs: Vec<ZethInput>,  // <-- Zeth format, not RSP!
    pub trusted_height: u64,
    pub trusted_root: FixedBytes<32>,
}
```

## For Prover Implementation

When implementing RISC0 backend in `ev-prover`, you have two options:

### Option 1: Fetch Data Twice (Recommended for Now)
- Fetch EVM execution data using Zeth's host library
- Build `Risc0BlockExecInput` with Zeth witnesses
- Use separate code paths for SP1 and RISC0

```rust
#[cfg(feature = "risc0")]
{
    // Use zeth-host to fetch execution witnesses
    let zeth_input = zeth_host::prepare_input(block_number, rpc_url)?;
    let risc0_input = Risc0BlockExecInput {
        // ... celestia data ...
        zeth_inputs: vec![zeth_input],
        // ...
    };
}
```

### Option 2: Implement RSP → Zeth Converter (Future Work)
- Create a converter that transforms RSP witness format to Zeth format
- Requires deep understanding of both RSP and Zeth internals
- Significant engineering effort but allows code reuse

## Comparison with SP1

| Aspect | SP1 | RISC0 |
|--------|-----|-------|
| Input Type | `BlockExecInput` | `Risc0BlockExecInput` |
| EVM Executor | RSP | Zeth |
| Witness Format | `EthClientExecutorInput` | `zeth_core::Input` |
| Reth Version | 1.5.0 | 1.7.0 |
| Data Fetching | `rsp-host-executor` | `zeth-host` (TBD) |

## Next Steps

1. ✅ Define `Risc0BlockExecInput` type
2. ✅ Implement RISC0 guest with Zeth
3. ⏸️ **CURRENT**: Document input type differences
4. 📝 **TODO**: Implement Zeth-based input preparation in `ev-prover`
5. 📝 **TODO**: Create e2e test with RISC0 backend

## Related Files

- `crates/risc0/ev-exec/guest/src/lib.rs` - Risc0BlockExecInput definition
- `crates/ev-zkevm-types/src/programs/block.rs` - BlockExecInput (SP1)
- `crates/risc0/RISC0_CONSTANTS.rs` - RISC0 program IDs
