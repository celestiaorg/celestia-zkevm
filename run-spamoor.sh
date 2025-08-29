#!/bin/bash

# Script to run spamoor transaction flooding against the reth service
# Uses the same funded account as the hyperlane setup

echo "Starting spamoor transaction flooding..."

docker run --rm -it \
  --network celestia-zkevm-hl-testnet_celestia-zkevm-net \
  -e RPC_URL=http://reth:8545 \
  -e PRIVATE_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
  ethpandaops/spamoor:latest \
  "$@"