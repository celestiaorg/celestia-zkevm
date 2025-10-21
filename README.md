# ZK EVM Hyperlane

> [!WARNING]
> This repository is a work in progress and under active development.

This repository showcases a bridged token transfer between Celestia and a ZK proveable EVM via [Hyperlane](https://hyperlane.xyz/). 
For more information refer to the [architecture](./docs/ARCHITECTURE.md). Note that the design is subject to change.

## Usage

### Preamble

#### Choosing a Proof System

This project supports two zero-knowledge proof systems:
- **SP1** (default): Succinct's SP1 zkVM
- **RISC0**: RISC Zero's zkVM

Both proof systems are optional and controlled by Cargo features. By default, only SP1 is enabled.

**To use SP1 (default):**
```bash
# Build with default features (SP1 only)
cargo build --release
```

**To use RISC0:**
```bash
# Build with RISC0 feature instead of SP1
cargo build --release --no-default-features --features risc0
```

**To enable both:**
```bash
# Build with both proof systems available
cargo build --release --features risc0
```

Once built with the desired features, select the active proof system at runtime using the `PROOF_SYSTEM` environment variable:

```env
# Use SP1 (if built with sp1 feature)
PROOF_SYSTEM=sp1

# Or use RISC0 (if built with risc0 feature)
PROOF_SYSTEM=risc0
```

If not specified, the system defaults to SP1 when available, otherwise RISC0.

#### SP1 Configuration

SP1 supports generating proofs in mock mode or network mode. By default, mock mode is used which is faster for testing and development purposes. Network mode is used for production purposes to generate real proofs. To use network mode, modify your `.env` file:

```env
SP1_PROVER=network
NETWORK_PRIVATE_KEY="PRIVATE_KEY" to the SP1 prover network private key from Celestia 1Password
```

#### RISC0 Configuration

RISC0 supports multiple proving modes similar to SP1:

```env
# Use local CPU proving (default for RISC0)
RISC0_PROVER=local

# Use Bonsai proving network (requires API key)
RISC0_PROVER=bonsai
BONSAI_API_KEY="your_api_key"
BONSAI_API_URL="https://api.bonsai.xyz/"

# Use GPU acceleration (if available)
RISC0_PROVER=cuda
```

**Proof Modes:**
- **Default/Core**: Fast proving, larger proof size (development/testing)
- **Groth16**: Slower proving, smallest proof size, fastest verification (production)

By default, RISC0 will use local CPU proving. For production deployments using Groth16 proofs with the Bonsai proving network, configure the environment variables above.

#### SP1 vs RISC0: Quick Comparison

| Feature | SP1 | RISC0 |
|---------|-----|-------|
| **Default Mode** | Mock (fastest, no proof) | Local CPU proving |
| **Network Proving** | `SP1_PROVER=network` | `RISC0_PROVER=bonsai` |
| **GPU Support** | `SP1_PROVER=cuda` | `RISC0_PROVER=cuda` |
| **Proof Formats** | Core, Compressed, Groth16, PLONK | Default, Groth16 |
| **Development Speed** | Fastest (mock mode) | Fast (local proving) |
| **Maturity** | Production-ready | Experimental integration |

For most use cases, **SP1 is recommended** as it's the default and has been more extensively tested in this project. RISC0 support is provided as an alternative backend for users who prefer that ecosystem.

### Prerequisites

1. Install [Docker](https://docs.docker.com/get-docker/)
2. Install [Foundry](https://book.getfoundry.sh/getting-started/installation)
3. Install [Rust](https://rustup.rs/)
4. Install a zkVM toolchain:
   - For SP1: Install [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)
   - For RISC0: Install [RISC0](https://dev.risczero.com/api/zkvm/install) via `curl -L https://risczero.com/install | bash && rzup install`

### Steps

1. Clone this repository.

    ```shell
    git clone git@github.com:celestiaorg/celestia-zkevm-hl-testnet.git
    ```

2. Source the provided `.env` file in this repository.

    ```shell
    cp .env.example .env

    set -a
    source .env
    set +a
    ```

3. Start the docker compose services.

    ```shell
    # Run `make start` or `docker compose up` from the root of the repository
    make start 
    ```

4. Allow the `hyperlane-init` service to complete the provisioning of Hyperlane EVM contracts and cosmosnative components.

    ```shell
    # Stream the logs via Docker to observe the status.
    docker logs -f hyperlane-init
    ```

5. Run a Hyperlane warp transfer, bridging `utia` from celestia to the reth service.

    ```shell
    make transfer
    ```

6. Query the ERC20 balance of the recipient on the reth service.

    ```shell
    make query-balance
    ```

7. Transfer funds back from the ERC20 contract to celestia.

    ```shell
    make transfer-back
    ```

8. Stop and teardown docker compose services.

    ```shell
    make stop
    ```

## Running the E2E test
1. Clone this repository.

    ```shell
    git clone git@github.com:celestiaorg/celestia-zkevm-hl-testnet.git
    ```

2. Source the provided `.env` file in this repository.

    ```shell
    cp .env.example .env

    set -a
    source .env
    set +a
    ```

3. Select a prover mode other than `mock`. Valid choices are `network`, `cuda`, `cpu`.
    in `.env`:
    ```shell
    SP1_PROVER=cpu #network, cuda
    ```

5. Use the prover service binary to generate config files locally.
    ```
    cargo run --bin ev-prover init
    ```
    Alternatively, install the client binary and run init:
    ```
    cargo install --path ./crates/ev-prover
    ev-prover init
    ```

6. Start the docker compose services.

    ```shell
    # Run `make start` or `docker compose up` from the root of the repository
    make start 
    ```

    Wait for all containers to finish their initialization sequence.

7. Run the e2e
    ```shell
    RUST_LOG="e2e=info" make e2e
    ```

    Note that depending on your hardware it can take a while for the e2e to run,
    as it will prove a series of EVM blocks leading up to a target height, as well as state inclusion of a Hyperlane deposit message at the target height.

## Architecture

See [ARCHITECTURE.md](./docs/ARCHITECTURE.md) for more information.

## Benchmarking

See [Benchmarks](./testdata/benchmarks/README.md) for more information.

## Context

The objective of this project is to establish a ZK bridge from the Celestia base-layer to a Celestia rollup and back. The sequencer of each rollup submits `tx blobs` to the base-layer that are used to build the EVM blocks, which are then verified against a previous, trusted EVM block from that same rollup. In order to facilitate transfers from one Celestia rollup to another, a forwarding module will be introduced to the base-layer in the future.

This ZK bridge is internal to the Celestia ecosystem, meaning that generic EVM chains, like Ethereum, which exist outside Celestia, will require a connection to the base-layer. This connection will usually be a ZK light client, such as `SP1-Helios`, that submits proofs and header roots to the base-layer's ISM module.

## Contributing

See [CONTRIBUTING.md](./docs/CONTRIBUTING.md) for more information.
