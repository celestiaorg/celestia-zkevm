#!/bin/bash

set -euo pipefail

CHAIN_ID="celestia-zkevm-testnet"
CONSENSUS_RPC="http://celestia-validator:26657/status"

if [ -f /home/celestia/.env ]; then
    echo "Skipping initialisation..."
    source /home/celestia/.env

    echo $CELESTIA_CUSTOM
else
    echo "Fetching genesis block hash at $CONSENSUS_RPC..."
    GEN_BLOCK_HASH=$(curl -sf "$CONSENSUS_RPC" | jq -r '.result.sync_info.earliest_block_hash')

    if [[ "$GEN_BLOCK_HASH" == "null" || -z "$GEN_BLOCK_HASH" ]]; then
        echo "Could not retrieve a valid genesis block hash from $CONSENSUS_RPC"
        exit 1
    fi

    echo "Exporting env CELESTIA_CUSTOM=$CHAIN_ID:$GEN_BLOCK_HASH"
    export CELESTIA_CUSTOM="$CHAIN_ID:$GEN_BLOCK_HASH"
    echo "export CELESTIA_CUSTOM=$CELESTIA_CUSTOM" > /home/celestia/.env

    echo "Initializing bridge node..."
    celestia bridge init \
        --core.ip celestia-validator \
        --rpc.addr 0.0.0.0 \
        --rpc.port 26658
fi

echo "Starting bridge node..."
exec "$@"