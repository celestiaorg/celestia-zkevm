## Hyperlane Local Registry

### Prerequisites

- Docker
- Hyperlane CLI

1. Install Hyperlane CLI

```
npm install -g @hyperlane-xyz/cli
```

2. Launch the docker compose services at the root of this repo.

```
docker compose up
```

### Deploy EVM Contracts

1. Configure an env variable `HYP_KEY` for tx signing.
Use the following privkey as the associated account is already funded in the docker-compose reth service genesis file.

```
export HYP_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a
```

2. Inspect the local hyperlane registry.

```
hyperlane registry list --registry ./hyperlane
```


3. Initialise a core deployment config using a TestISM (NoopISM).
This step requires the `--advanced` flag. Follow the instructions by the prompt.

NOTE: This step can be skipped as we will add `configs` to version control.

```
hyperlane core init --advanced --registry ./hyperlane
```

4. Deploy the core contracts on Reth.

```
hyperlane core deploy --chain rethlocal --registry ./hyperlane
```
