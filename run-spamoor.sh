#!/bin/bash

# Script to run spamoor transaction flooding against the reth service
# Uses the PRIVATE_KEY environment variable from .env file

# Check if PRIVATE_KEY is set
if [ -z "$PRIVATE_KEY" ]; then
    echo "Error: PRIVATE_KEY environment variable is not set."
    echo "Please source your .env file: source .env"
    exit 1
fi

echo "Starting spamoor transaction flooding..."

docker run --rm -it \
  --network celestia-zkevm-hl-testnet_celestia-zkevm-net \
  -e RPC_URL=http://reth:8545 \
  -e PRIVATE_KEY="$PRIVATE_KEY" \
  ethpandaops/spamoor:latest \
  "$@"