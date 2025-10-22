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

## ✅ Implemented Solution

The prover now uses **dual-path data fetching** with feature-gated compilation:

### Implementation Details

**SP1 Path** (`#[cfg(all(feature = "sp1", not(feature = "risc0")))]`):
- Uses `rsp-host-executor::EthHostExecutor` to fetch execution witnesses
- Creates `BlockExecInput` with `EthClientExecutorInput` (RSP format)
- Calls `eth_client_executor_input()` method

**RISC0 Path** (`#[cfg(all(feature = "risc0", not(feature = "sp1")))]`):
- Uses `zeth-host::BlockProcessor` to fetch execution witnesses
- Creates `Risc0BlockExecInput` with `zeth_core::Input` (Zeth format)
- Calls `zeth_input()` method

### Code Example

```rust
// RISC0-specific input fetching (implemented in block.rs)
#[cfg(feature = "risc0")]
async fn zeth_input(&self, block_number: u64) -> Result<ZethInput> {
    let provider = ProviderBuilder::new().connect_http(self.app.evm_rpc.parse()?);
    let block_processor = BlockProcessor::new(provider).await?;
    let (input, _block_hash) = block_processor.create_input(block_number).await?;
    Ok(input)
}
```

### Why Not Option 2 (RSP → Zeth Converter)?
- **Complexity**: RSP and Zeth use fundamentally different witness structures
- **Maintenance**: Would require keeping converter in sync with both libraries
- **Performance**: Dual fetching is simpler and more maintainable
- **Flexibility**: Each backend can optimize its own data fetching path

## Comparison with SP1

| Aspect | SP1 | RISC0 |
|--------|-----|-------|
| Input Type | `BlockExecInput` | `Risc0BlockExecInput` |
| EVM Executor | RSP | Zeth |
| Witness Format | `EthClientExecutorInput` | `zeth_core::Input` |
| Reth Version | 1.5.0 | 1.7.0 |
| Data Fetching | `rsp-host-executor` | `zeth-host::BlockProcessor` ✅ |
| RPC Requirement | Standard Ethereum RPC | RPC with `debug_executionWitness` |

## Implementation Status

1. ✅ Define `Risc0BlockExecInput` type
2. ✅ Implement RISC0 guest with Zeth
3. ✅ Document input type differences
4. ✅ **DONE**: Implement Zeth-based input preparation in `ev-prover`
5. ⏳ **NEXT**: Create e2e test with RISC0 backend (requires RPC with `debug_executionWitness`)

## Related Files

- `crates/risc0/ev-exec/guest/src/lib.rs` - Risc0BlockExecInput definition
- `crates/ev-zkevm-types/src/programs/block.rs` - BlockExecInput (SP1)
- `crates/risc0/RISC0_CONSTANTS.rs` - RISC0 program IDs
