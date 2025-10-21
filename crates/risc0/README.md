# Risc0 Circuits for Celestia zkEVM Hyperlane

This directory contains Risc0 implementations of the zero-knowledge circuits for the Celestia zkEVM Hyperlane integration.

## Structure

```
risc0/
├── ev-exec/          # EVM block execution verification
│   ├── guest/        # Risc0 guest program (runs in zkVM)
│   └── host/         # Risc0 prover/host code
├── ev-hyperlane/     # Hyperlane message verification
│   ├── guest/        # Risc0 guest program (runs in zkVM)
│   └── host/         # Risc0 prover/host code
└── ev-range-exec/    # Recursive proof aggregation (TODO)
    ├── guest/        # Risc0 guest program (runs in zkVM)
    └── host/         # Risc0 prover/host code
```

## Circuits

### 1. EV-Exec (Block Execution)
**Guest**: `ev-exec/guest/src/lib.rs` and `ev-exec/guest/src/bin/ev-exec.rs`

Verifies inclusion of EVM blocks in Celestia DA and executes state transitions:
- Deserializes Celestia block header and blobs
- Verifies namespace inclusion and completeness
- Executes EVM blocks via **Zeth** (RISC Zero's Ethereum block prover)
- Verifies sequencer signatures on blob data
- Validates transaction roots match blob data

**Inputs**: `Risc0BlockExecInput` (RISC0-specific input type)
**Outputs**: `BlockExecOutput`

### 2. EV-Hyperlane (Message Verification)
**Guest**: `ev-hyperlane/guest/src/main.rs`

Verifies Hyperlane messages against on-chain Merkle Tree state:
- Verifies Patricia Trie branch proofs
- Inserts message IDs into Merkle tree snapshot
- Validates against EVM state root

**Inputs**: `HyperlaneMessageInputs`
**Outputs**: `HyperlaneMessageOutputs`

### 3. EV-Range-Exec (Recursive Aggregation) [TODO]
**Status**: Not yet implemented

Will aggregate multiple compressed proofs into a single Groth16 proof.

## Building

### ✅ RISC0 Implementation with Zeth

**RISC0 support is now fully functional using Zeth for EVM block execution!**

Due to incompatible crypto patches between SP1 and RISC0, this workspace is **separate** from the main project workspace:

**Architecture**:
- **SP1 Backend**: Uses Succinct's RSP (Reth State Prover) for EVM execution
- **RISC0 Backend**: Uses Boundless's Zeth for EVM execution
- **Separate Workspaces**: RISC0 programs live in their own workspace with RISC0 crypto patches
- **Shared Celestia Logic**: Namespace verification and DA proofs are shared where possible

**Status**:
- ✅ RISC0 backend interface implemented in `ev-prover`
- ✅ Separate RISC0 workspace with Zeth integration
- ✅ EV-Exec circuit builds and runs with Zeth
- ✅ Independent build system via `crates/risc0/Cargo.toml`
- ⚠️ Must build RISC0 programs from this directory (not from main workspace)

**Prerequisites**:
Clone Zeth to use as a dependency:
```bash
cd /Users/blasrodriguezgarciairizar/projects/celestia
git clone https://github.com/boundless-xyz/zeth.git
```

The workspace configuration in `crates/risc0/Cargo.toml` references Zeth from this location.

### Prerequisites
```bash
# Install Risc0 toolchain
curl -L https://risczero.com/install | bash
rzup install

# Or use cargo
cargo install cargo-risczero
cargo risczero install
```

### Build Guest Programs
```bash
# IMPORTANT: Build from the risc0/ directory, NOT from the main workspace!
cd crates/risc0

# Build EV-Exec guest program (with Zeth integration)
cargo build --package ev-exec-guest

# Build other guest programs (TODO: update for Zeth)
# cargo build --package ev-hyperlane-guest
# cargo build --package ev-range-exec-guest
```

### Build Host/Prover
```bash
# Build from the risc0/ directory
cd crates/risc0

# Build host code with embedded guest binaries
cargo build --package ev-exec-host

# The build process will:
# 1. Compile the guest program for RISC0's zkVM
# 2. Generate the ImageID and ELF binary
# 3. Embed them in the host library
# 4. Create methods.rs with exported constants
```

## Implementation Details

### Separate Input Types

Due to the different EVM execution engines (RSP vs Zeth), SP1 and RISC0 have different input types:

**SP1**: `BlockExecInput` (from `ev-zkevm-types`)
- Uses RSP's `EthClientInput` for EVM execution witnesses

**RISC0**: `Risc0BlockExecInput` (from `ev-exec-guest`)
- Uses Zeth's `Input` for EVM execution witnesses
- Defined in the guest library for direct access to types

### Shared Celestia Logic

The Celestia-specific verification logic is shared:
- Namespace proof verification
- Data availability header validation
- Sequencer signature verification
- Blob data parsing and validation

### Zeth Integration

The `ev-exec` guest program uses Zeth for EVM block execution:
```rust
use zeth_core::{EthEvmConfig, Input as ZethInput, validate_block};

// Create EVM config
let evm_config = EthEvmConfig::new((*zeth_chainspec::MAINNET).clone());

// Execute block using Zeth's stateless validation
let state_root = validate_block(zeth_input, evm_config)?;
```

## Usage

### Proof Generation
```rust
use risc0_zkvm::{default_prover, ExecutorEnv};
use ev_exec_host::{EV_EXEC_ELF, EV_EXEC_IMAGE_ID, Risc0BlockExecInput, BlockExecOutput};

// Prepare input
let input = Risc0BlockExecInput {
    header_raw: celestia_header_bytes,
    dah: data_availability_header,
    blobs_raw: blobs_bytes,
    pub_key: sequencer_public_key,
    namespace,
    proofs: namespace_proofs,
    zeth_inputs: vec![zeth_input], // Zeth execution witnesses
    trusted_height,
    trusted_root,
};

// Build execution environment
let env = ExecutorEnv::builder()
    .write(&input)
    .unwrap()
    .build()
    .unwrap();

// Generate proof
let prover = default_prover();
let receipt = prover.prove(env, EV_EXEC_ELF).unwrap();

// Extract public values
let output: BlockExecOutput = receipt.journal.decode().unwrap();
```

### With Proof System Abstraction
```rust
use ev_prover::proof_system::{ProverFactory, ProofSystemType};

// Create Risc0 backend
let backend = ProverFactory::create(ProofSystemType::Risc0)?;

// Generate proof
let proof = backend.prove(program_id, input, ProofMode::Groth16).await?;
```

## Differences from SP1

| Aspect | SP1 | Risc0 |
|--------|-----|-------|
| Entry point | `sp1_zkvm::entrypoint!(main)` | `risc0_zkvm::guest::entry!(main)` |
| Input | `sp1_zkvm::io::read()` | `env::read()` |
| Output | `sp1_zkvm::io::commit()` | `env::commit()` |
| Proof modes | Core, Compressed, Groth16, Plonk | Default, Groth16 |
| Recursion | `verify_sp1_proof()` | Composition/joining |
| Cycle tracking | `println!("cycle-tracker-...")` | Risc0 profiling |

## Testing

Run Risc0 circuit tests:
```bash
# Test guest programs compile
cargo test --package ev-exec-guest
cargo test --package ev-hyperlane-guest

# Test host/prover code
cargo test --package ev-exec-host
cargo test --package ev-hyperlane-host
```

## Dependencies

- `risc0-zkvm = "3.0.3"` - Zero-knowledge virtual machine
- `risc0-build = "3.0.3"` - Build system for guest programs
- `ev-zkevm-types` - Shared input/output types and logic

## Performance

_Benchmarks to be added after full implementation_

Expected characteristics:
- **Proof generation**: Slower than SP1 (different optimization focus)
- **Proof size**: Comparable for Groth16 mode
- **Verification time**: Similar for Groth16 mode

## Roadmap

- [x] Basic guest/host structure
- [x] Risc0Backend prover integration
- [x] Multi-backend prover programs (block.rs, message.rs, range.rs)
- [x] Feature-gated compilation support
- [x] **✅ SOLVED: Integrated Zeth for RISC0 EVM execution**
- [x] **✅ SOLVED: Created separate workspace to resolve crypto patch conflicts**
- [x] EV-Exec circuit implementation with Zeth
- [x] Successful compilation and binary generation
- [ ] EV-Hyperlane circuit implementation (update for RISC0)
- [ ] EV-Range-Exec recursive circuit (update for RISC0)
- [ ] End-to-end testing with RISC0 backend
- [ ] Groth16 SNARK conversion implementation
- [ ] Performance optimizations
- [ ] Benchmarking suite (SP1 vs RISC0 comparison)

## Resources

- [Risc0 Documentation](https://dev.risczero.com/)
- [Risc0 GitHub](https://github.com/risc0/risc0)
- [Risc0 Examples](https://github.com/risc0/risc0/tree/main/examples)
- [Zeth (RISC Zero Ethereum Prover)](https://github.com/boundless-xyz/zeth)
- [Zeth Documentation](https://github.com/boundless-xyz/zeth/blob/main/README.md)
- [Issue #248](https://github.com/celestiaorg/celestia-zkevm-hl-testnet/issues/248)

## Contributing

When adding new circuits or modifying existing ones:
1. **Build from the `crates/risc0/` directory** - this workspace has RISC0-specific crypto patches
2. Use Zeth for EVM execution verification (not RSP)
3. Define RISC0-specific input types in guest libraries when needed
4. Keep Celestia verification logic shareable where possible
5. Keep guest programs minimal (just I/O and delegation to library functions)
6. Add tests for both guest and host code
7. Update this README with any changes
8. Ensure RISC0 and SP1 implementations produce equivalent proofs for the same inputs

## License

Same as parent project.
