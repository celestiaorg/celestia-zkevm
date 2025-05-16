## Hyperlane Deployment

### Prerequisites

- Docker
- Foundry
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
hyperlane core deploy --chain rethlocal --registry ./hyperlane --yes
```

5. Create synthetic token on Reth.

```
hyperlane warp deploy --config ./configs/warp-config.yaml --registry ./hyperlane/
```

### Deploy Hyperlane core ISM and Mailbox on Celestia

To automate this run `go run main.go` from this directory.

NOTE: Here we run the `celestia-appd` txs from inside the docker compose service container.
This is mainly for access to the keyring without having to recover keys on the host machine. 
This is not strictly required, and will aim to automate this later.

1. Create a `NoopISM`

```
celestia-appd tx hyperlane ism create-noop --from default --fees 400utia
```

2. Create a `Mailbox`

The ID provided is the ISM ID.

```
celestia-appd tx hyperlane mailbox create 0x726f757465725f69736d00000000000000000000000000000000000000000000 69420 --from default --fees 400utia
```

3. Create `Hooks`. For testing we will first create `NoopHooks`.

```
celestia-appd tx hyperlane hooks noop create --from default --fees 400utia
```

4. Set the hooks on the mailbox

```
celestia-appd tx hyperlane mailbox set $MAILBOX --required-hook $HOOKS --default-hook $HOOKS --from default --fees 400utia
```

5. Create a `utia` collateral token.

```
celestia-appd tx warp create-collateral-token [mailbox-id] utia --from default  --fees 400utia
```

6. Set the default ISM on the collateral token.

```
celestia-appd tx warp set-token 0x726f757465725f61707000000000000000000000000000010000000000000000 --ism-id $ismID --from default --fees 400utia
```

### Enroll remote routers for both collateral and synthetic tokens

1. Enroll the synethetic token contract on Reth as the remote router contract on the celestia-app cosmosnative module.
NOTE: Here we left-pad the 20byte EVM address to conform to the `HexAddress` spec of cosmosnative.
It is unclear whether or not this is the correct approach. See: https://github.com/bcp-innovations/hyperlane-cosmos/issues/121

```
celestia-appd tx warp enroll-remote-router 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000a7578551baE89a96C3365b93493AD2D4EBcbAe97 0 --from default --fees 400utia
```

2. Enroll the collateral token ID from the celestia-app cosmosnative module as the remote router on the synthetic token contract (EVM).
Normally this should be possible to configure in a `warp-config.yaml` using the hyperlane CLI however, there isn't cosmosnative support yet.
Instead, we attempt to do this manually by invoking the EVM contract directly.

```
cast send 0xa7578551baE89a96C3365b93493AD2D4EBcbAe97 \
  "enrollRemoteRouter(uint32,bytes32)" \
  69420 0x726f757465725f61707000000000000000000000000000010000000000000000 \
  --private-key $HYP_KEY \
  --rpc-url http://localhost:8545
```

Validate the above tx succeeded by running the following query.

```
cast call 0xa7578551baE89a96C3365b93493AD2D4EBcbAe97 \
  "routers(uint32)(bytes32)" 69420 \
  --rpc-url http://localhost:8545

0x726f757465725f61707000000000000000000000000000010000000000000000
```

TODO: Try to get the relayer working and successfully transfer utia to evm

```
celestia-appd tx warp transfer 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000d7958B336f0019081Ad2279B2B7B7c3f744Bce0a "1000" --from default --fees 400utia --max-hyperlane-fee 100utia
```

Querying ERC20 balanceOf method of synthetic token contract:

```
cast call 0xa7578551baE89a96C3365b93493AD2D4EBcbAe97 \
  "balanceOf(address)(uint256)" \
  0xd7958B336f0019081Ad2279B2B7B7c3f744Bce0a \
  --rpc-url http://localhost:8545
```