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
**Guest**: `ev-exec/guest/src/main.rs`

Verifies inclusion of EVM blocks in Celestia DA and executes state transitions:
- Deserializes Celestia block header and blobs
- Verifies namespace inclusion and completeness
- Executes EVM blocks via Reth State Prover (RSP)
- Verifies sequencer signatures on blob data
- Validates transaction roots match blob data

**Inputs**: `BlockExecInput` (from `ev-zkevm-types`)
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

### ⚠️ Important: Workspace Limitation

**RISC0 guest/host crates are currently excluded from the main workspace** due to conflicting cryptographic library patches between SP1 and RISC0. Both proof systems use patched versions of `k256`, `sha2`, and other crypto libraries, but the patches are incompatible.

**Current Status**: The RISC0 backend implementation in `ev-prover` is complete and functional, but building RISC0 guest programs requires a separate workspace setup.

**Workaround**: Create a separate Cargo workspace for RISC0 crates or build them individually with manual dependency management.

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
# NOTE: These commands will currently fail when run from the main workspace
# due to SP1 crypto patches. A separate workspace is needed.

# Build all Risc0 guest programs
cargo build --package ev-exec-guest
cargo build --package ev-hyperlane-guest
```

### Build Host/Prover
```bash
# NOTE: These commands will currently fail when run from the main workspace
# due to SP1 crypto patches. A separate workspace is needed.

# Build host code with embedded guest binaries
cargo build --package ev-exec-host
cargo build --package ev-hyperlane-host
```

## Shared Logic

All circuit logic is factored into `ev-zkevm-types`, a pure Rust crate with no zkVM dependencies. This allows the same verification logic to be used by both SP1 and Risc0 implementations.

**Shared modules:**
- `ev-zkevm-types::programs::block` - Block execution logic
- `ev-zkevm-types::programs::hyperlane` - Message verification logic

## Usage

### Proof Generation
```rust
use risc0_zkvm::{default_prover, ExecutorEnv};
use ev_exec_host::EV_EXEC_GUEST_ID;

// Build execution environment
let env = ExecutorEnv::builder()
    .write(&input)
    .unwrap()
    .build()
    .unwrap();

// Generate proof
let prover = default_prover();
let receipt = prover.prove(env, EV_EXEC_GUEST_ID).unwrap();

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
- [x] EV-Exec circuit implementation
- [x] EV-Hyperlane circuit implementation
- [x] EV-Range-Exec recursive circuit
- [x] Risc0Backend prover integration
- [x] Multi-backend prover programs (block.rs, message.rs, range.rs)
- [x] Feature-gated compilation support
- [ ] **Resolve workspace crypto patch conflicts** (blocking)
- [ ] End-to-end testing with RISC0 backend
- [ ] Groth16 SNARK conversion implementation
- [ ] Performance optimizations
- [ ] Benchmarking suite (SP1 vs RISC0 comparison)

## Resources

- [Risc0 Documentation](https://dev.risczero.com/)
- [Risc0 GitHub](https://github.com/risc0/risc0)
- [Risc0 Examples](https://github.com/risc0/risc0/tree/main/examples)
- [Issue #248](https://github.com/celestiaorg/celestia-zkevm-hl-testnet/issues/248)

## Contributing

When adding new circuits or modifying existing ones:
1. Keep logic in `ev-zkevm-types` when possible
2. Keep guest programs minimal (just I/O and delegation)
3. Add tests for both guest and host code
4. Update this README with any changes
5. Ensure compatibility with SP1 implementation

## License

Same as parent project.
