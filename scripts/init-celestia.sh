#!/bin/sh

GENESIS_FILE="/home/celestia/.celestia-appd/config/genesis.json"

if [ ! -f "$GENESIS_FILE" ]; then
    echo "Initializing Celestia App state..."

    celestia-appd init zkevm-test --chain-id celestia-zkevm-testnet
    celestia-appd config set client chain-id celestia-zkevm-testnet
    celestia-appd config set client keyring-backend test

    celestia-appd keys add default
    celestia-appd keys add validator

    # TODO: Remove both overrides when image is updated to version with fixes.
    echo "Overriding expedited_min_deposit to 50,000 TIA..."
    jq '.app_state.gov.params.expedited_min_deposit |= map(if .denom == "utia" then .amount = "50000000000" else . end)' "$GENESIS_FILE" > tmp && mv tmp "$GENESIS_FILE"

    echo "Setting app version to 4..."
    jq '.consensus.params.version.app = "4"' "$GENESIS_FILE" > tmp && mv tmp "$GENESIS_FILE"

    celestia-appd genesis add-genesis-account "$(celestia-appd keys show default -a)" 1000000000000utia
    celestia-appd genesis add-genesis-account "$(celestia-appd keys show validator -a)" 1000000000000utia
    celestia-appd genesis gentx validator 100000000utia --fees 500utia
    celestia-appd genesis collect-gentxs
    celestia-appd genesis validate

    echo "Successfully initialized chain state."
else
    echo "Skipping init, genesis.json already exists."
fi