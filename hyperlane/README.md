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

### Deploy EVM core and warp route contracts

1. Configure an env variable `HYP_KEY` for tx signing.
Use the following privkey as the associated account is already funded in the docker-compose reth service genesis file.

```
export HYP_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a
```

2. Inspect the local hyperlane registry.

```
hyperlane registry list --registry ./registry
```

3. Initialise a core deployment config using a TestISM (NoopISM).
This step requires the `--advanced` flag. Follow the instructions by the prompt.

NOTE: This step can be skipped as we will add `configs` to version control.

```
hyperlane core init --advanced --registry ./registry
```

4. Deploy the core contracts on Reth.

NOTE: Uses `./configs/core-config.yaml` by default.

```
hyperlane core deploy --chain rethlocal --registry ./registry --yes
```

5. Create synthetic token on Reth.

NOTE: Here we must specify the `--config` flag to the warp router deployment config.

```
hyperlane warp deploy --config ./configs/warp-config.yaml --registry ./registry --yes
```

### Deploy Cosmosnative core and warp route on Celestia

> As there is no cosmosnative support in the hyperlane CLI, a Go program has been added to automate this.

```
go install ./cmd/hyp

# Deploy with NoopISM (for testing)
hyp deploy-noopism 127.0.0.1:9090 69420

# Deploy with ZK Execution ISM
hyp deploy-zkism 127.0.0.1:9090 [evm-rpc] [ev-node-rpc] 69420

# Deploy with Multisig ISM
hyp deploy-multisigism 127.0.0.1:9090 [validators-comma-separated] [threshold] 69420
```

The local-domain parameter (e.g., `69420`) specifies the domain ID for the Celestia chain.

Below is a list of the manual steps which are performed by the Go program used above.
Skip to the next section to configure the remote routers for both the EVM and cosmosnative deployments.

1. Create a `NoopISM`

```
celestia-appd tx hyperlane ism create-noop --from default --fees 800utia
```

2. Create a `Mailbox`

The ID provided is the ISM ID.

```
celestia-appd tx hyperlane mailbox create 0x726f757465725f69736d00000000000000000000000000000000000000000000 69420 --from default --fees 800utia
```

3. Create `Hooks`. For testing we will first create `NoopHooks`.

```
celestia-appd tx hyperlane hooks noop create --from default --fees 800utia
```

4. Set the hooks on the mailbox

```
celestia-appd tx hyperlane mailbox set $MAILBOX --required-hook $HOOKS --default-hook $HOOKS --from default --fees 800utia
```

5. Create a `utia` collateral token.

```
celestia-appd tx warp create-collateral-token [mailbox-id] utia --from default  --fees 800utia
```

6. Set the default ISM on the collateral token.

```
celestia-appd tx warp set-token 0x726f757465725f61707000000000000000000000000000010000000000000000 --ism-id $ismID --from default --fees 800utia
```

### Enroll remote routers for both collateral and synthetic tokens

Now that we've deployed the Hyperlane core and warp route infrastructure for a collateral token on Celestia and a synthetic token on Reth, 
we must establish a link between the two tokens and mailboxes.

1. Enroll the synthetic token contract on Reth as the remote router contract on the celestia-app cosmosnative module.
NOTE: Here we left-pad the 20byte EVM address to conform to the `HexAddress` spec of cosmosnative.

NOTE: The following can be run from inside the `celestia-validator` service container, or from your host machine if you have access to a key for a funded account.

```
celestia-appd tx warp enroll-remote-router [token-id] [remote-domain] [receiver-contract] [gas]

celestia-appd tx warp enroll-remote-router 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000a7578551baE89a96C3365b93493AD2D4EBcbAe97 0 --from hyp --fees 800utia
```

Validate the above tx succeeded by running the following query:

```
celestia-appd q warp remote-routers 0x726f757465725f61707000000000000000000000000000010000000000000000
```

2. Enroll the collateral token ID from the celestia-app cosmosnative module as the remote router on the synthetic token contract (EVM).
Normally this should be possible to configure in a `warp-config.yaml` using the hyperlane CLI however, there isn't cosmosnative support yet.
Instead, we attempt to do this manually by invoking the EVM contract directly.

```
cast send 0x345a583028762De4d733852c9D4f419077093A48 \
  "enrollRemoteRouter(uint32,bytes32)" \
  69420 0x726f757465725f61707000000000000000000000000000010000000000000000 \
  --private-key $HYP_KEY \
  --rpc-url http://localhost:8545
```

Validate the above tx succeeded by running the following query.

```
cast call 0x345a583028762De4d733852c9D4f419077093A48 \
  "routers(uint32)(bytes32)" 69420 \
  --rpc-url http://localhost:8545

0x726f757465725f61707000000000000000000000000000010000000000000000
```

### Warp token transfer with Hyperlane relayer

Clone the [hyperlane-monorepo](https://github.com/hyperlane-xyz/hyperlane-monorepo) and navigate to `rust/main` and follow the instructions to build the `relayer` binary on the README.md.
There is a relayer config ready to use available in this directory: `relayer-config.json`. Configure it using the `CONFIG_FILES` env variable.

For example, drop the `relayer` binary into a directory called `bin` in this repo and then move the config to `bin/config/config.json`.
Then:

```
export CONFIG_FILES=./config/config.json

cd bin

./relayer
```

Exec into the `celestia-validator` container for access to `default` acc on the keyring.

```
docker exec -it celestia-validator /bin/bash
```

Run the `warp transfer` command. 

```
celestia-appd tx warp transfer 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000d7958B336f0019081Ad2279B2B7B7c3f744Bce0a "1000" --from default --fees 800utia --max-hyperlane-fee 100utia
```

Querying ERC20 balanceOf method of synthetic token contract:

```
cast call 0x345a583028762De4d733852c9D4f419077093A48 \
  "balanceOf(address)(uint256)" \
  0xd7958B336f0019081Ad2279B2B7B7c3f744Bce0a \
  --rpc-url http://localhost:8545
```

### Deploying a Warp route using native EVM token.

#### Deploy a `HypNative` token on Evolve

This guide will walkthrough the steps to deploy a `HypNative` token on EVM and a corresponding `Synthetic` token on Celestia and 
establish a link between the two.

Ensure you have a valid private key set for deploying EVM contracts under the env variable `HYP_KEY`.
Using the `hyperlane` CLI create a new Warp route configuration file by running the following:
```
hyperlane warp init --registry ./hyperlane/registry
```

Follow the instructions presented on screen. 
Example using the docker compose network in this repository:
1. Select the chain `rethlocal` from the testnet section.
2. Enter the desired owner address, e.g. `0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d`.
3. Select `No` for proxy admin contract.
4. Select `No` for the trusted relayer ism.
5. Select `Native` from the token list.

Deploy the Warp contract using the `hyperlane` CLI. Provide the flag `--symbol ETH` to refer to the file created under `deployments/ETH` from the initial step. Alternatively provide 
Note, if running the CLI locally against the docker network then you will need to adjust the `rpcUrl` in `registry/chains` to use localhost.
```
hyperlane warp deploy --registry ./hyperlane/registry --warp  --symbol ETH
```

#### Deploy a `Synthetic` token on Celestia

Create a new `Synthetic` token using the `celestia-appd` binary. 
```
celestia-appd tx warp create-synthetic-token 0x68797065726c616e650000000000000000000000000000000000000000000000 --from hyp --fees 800utia
```

Set the ism identifier for the new token on Celestia. By default the docker network will use the mailbox default ism, which is a `NoopISM`.
Here we set the `Synthetic` token ism to be the zk ism verifier.
```
celestia-appd tx warp set-token 0x726f757465725f61707000000000000000000000000000020000000000000001 --ism-id 0x726f757465725f69736d000000000000000000000000002a0000000000000001 --from hyp --fees 800utia
```

Query warp tokens using the `celestia-appd` binary.
```
celestia-appd q warp tokens
```

#### Set the remote routers to establish a link between both tokens

Finally we will set the remote router for both new tokens. Firstly on the EVM side and then on Celestia, the order in which this is done is does not matter.

Using `cast` we can invoke the contract directly to enroll the remote router providing both the `domain` and `Synthetic` token identifier.
```
cast send 0x032e1B988eB5Ac8F0C8617E09de92a664cABf37D \
  "enrollRemoteRouter(uint32,bytes32)" \
  69420 0x726f757465725f61707000000000000000000000000000020000000000000001 \
  --private-key $HYP_KEY \
  --rpc-url http://localhost:8545
```

Query the contract to ensure the previous transaction succeeded.
```
cast call 0x032e1B988eB5Ac8F0C8617E09de92a664cABf37D \
  "routers(uint32)(bytes32)" 69420
```

Enroll the `HypNative` contract as the remote router on the Celestia `Synthetic` token.
```
celestia-appd tx warp enroll-remote-router 0x726f757465725f61707000000000000000000000000000020000000000000001 1234 0x000000000000000000000000032e1B988eB5Ac8F0C8617E09de92a664cABf37D 0 --from hyp --fees 800utia
```

Query the token to ensure the remote router has been set successfully.
```
celestia-appd q warp remote-routers 0x726f757465725f61707000000000000000000000000000020000000000000001
```

#### Sending Evolve ETH to Celestia

Invoke the `transferRemote` method of the `HypNative` contract.
```
cast send 0x032e1B988eB5Ac8F0C8617E09de92a664cABf37D \
  	"transferRemote(uint32, bytes32, uint256)(bytes32)" \
  	69420 0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7 10000000 \
		--private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
  	--rpc-url http://localhost:8545 --value 10000000
```