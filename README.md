# ZK EVM Hyperlane

> [!WARNING]
> This repository is a work in progress and under active development.

This repository showcases a bridged token transfer between Celestia and a ZK proveable EVM via [Hyperlane](https://hyperlane.xyz/). 
For more information refer to the [architecture](./docs/ARCHITECTURE.md). Note that the design is subject to change.

## Usage

### Preamble

SP1 supports generating proofs in mock mode or network mode. By default, mock mode is used which is faster for testing and development purposes. Network mode is used for production purposes to generate real proofs. To use network mode, modify your `.env` file:

```env
SP1_PROVER=network
NETWORK_PRIVATE_KEY="PRIVATE_KEY" to the SP1 prover network private key from Celestia 1Password
```

### Prerequisites

1. Install [Docker](https://docs.docker.com/get-docker/)
2. Install [Foundry](https://book.getfoundry.sh/getting-started/installation)
3. Install [Rust](https://rustup.rs/)
4. Install [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)

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

## Architecture

See [ARCHITECTURE.md](./docs/ARCHITECTURE.md) for more information.

## Benchmarking

See [Benchmarks](./testdata/benchmarks/README.md) for more information.

## Context

The objective of this project is to establish a ZK bridge from the Celestia base-layer to a Celestia rollup and back. The sequencer of each rollup submits `tx blobs` to the base layer that are used to build the EVM blocks which are then verified against a previous, trusted EVM block. In order to faciliate transfers from one Celestia rollup to another, a forwarding module will be introduced to the base-layer. 

This ZK bridge is internal to the Celestia ecosystem, meaning that generic EVM chains, like Ethereum, which exist outside Celestia, will require a connection to the base-layer. This connection will usually be a ZK light-client like `SP1-Helios` that submits proofs and header roots to the base-layer's ISM module.

## Contributing

See [CONTRIBUTING.md](./docs/CONTRIBUTING.md) for more information.
