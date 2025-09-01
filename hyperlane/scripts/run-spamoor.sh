#!/bin/bash

# Script to run spamoor transaction flooding against the reth service
# Uses the PRIVATE_KEY environment variable from .env file and config file

# Check if PRIVATE_KEY is set
if [ -z "$PRIVATE_KEY" ]; then
    echo "Error: PRIVATE_KEY environment variable is not set."
    echo "Please source your .env file: source .env"
    exit 1
fi

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/../configs/spamoor-config.yaml"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file not found at $CONFIG_FILE"
    exit 1
fi

echo "Starting spamoor transaction flooding with config: $CONFIG_FILE"

docker run --rm -it \
  --network celestia-zkevm-hl-testnet_celestia-zkevm-net \
  -e RPC_URL=http://reth:8545 \
  -e PRIVATE_KEY="$PRIVATE_KEY" \
  -v "$CONFIG_FILE:/app/config.yaml:ro" \
  ethpandaops/spamoor:latest \
  run /app/config.yaml --rpchost http://reth:8545 --privkey "$PRIVATE_KEY" "$@"