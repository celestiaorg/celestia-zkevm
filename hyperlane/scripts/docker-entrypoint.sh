#!/bin/bash

# The following docker-entrypoint script performs deployment of Hyperlane infrastructure 
# on both ev-reth and celestia.
# To minimise proving time in the docker env in this repository we first deploy
# a noop ism stack on celestia and finally overwrite this with a new zk ism deployment.
# This ensures that the initial trusted root used in the zk ism is the same as the 
# latest block's state root in ev-reth.

set -euo pipefail

# HYP_KEY is the priv key of the EVM account used for Hyperlane contract deployment
export HYP_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a

CONFIG_FILE="hyperlane-cosmosnative.json"

if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "Using Hyperlane registry:"
  hyperlane registry list --registry ./registry

  echo "Deploying Hyperlane core EVM contracts..."
  hyperlane core deploy --chain rethlocal --registry ./registry --yes

  echo "Deploying Hyperlane warp synthetic token EVM contracts..."
  hyperlane warp deploy --config ./configs/warp-config.yaml --registry ./registry --yes

  echo "Deploying Hyperlane NoopISM stack on cosmosnative..."
  hyp deploy-noopism celestia-validator:9090

  echo "Configuring remote router for warp route on EVM..."
  cast send 0x345a583028762De4d733852c9D4f419077093A48 \
    "enrollRemoteRouter(uint32,bytes32)" \
    69420 0x726f757465725f61707000000000000000000000000000010000000000000000 \
    --private-key $HYP_KEY \
    --rpc-url http://reth:8545

  router_addr=$(cast call 0x345a583028762De4d733852c9D4f419077093A48 \
    "routers(uint32)(bytes32)" 69420 \
    --rpc-url http://reth:8545)

  echo "Successfully registered remote router address for domain 69420: $router_addr"

  echo "Configuring remote router for warp route on cosmosnative..."
  hyp enroll-remote-router celestia-validator:9090 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000345a583028762De4d733852c9D4f419077093A48

else
  echo "Skipping deployment: $CONFIG_FILE already exists."
fi