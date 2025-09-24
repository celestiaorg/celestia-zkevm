#!/bin/sh
set -e

cd /usr/bin

sleep 5

# Create default evnode config if missing
# TODO: The --evnode.signer.path flag is not respected: https://github.com/evstack/ev-node/issues/2603
if [ ! -f "$HOME/.evm-single/config/signer.json" ]; then
  ./evm-single init --evnode.node.aggregator=true --evnode.signer.passphrase $EVM_SIGNER_PASSPHRASE
fi

exec ./evm-single start \
  --evm.jwt-secret $EVM_JWT_SECRET \
  --evm.genesis-hash $EVM_GENESIS_HASH \
  --evm.engine-url $EVM_ENGINE_URL \
  --evm.eth-url $EVM_ETH_URL \
  --evnode.node.block_time $EVM_BLOCK_TIME \
  --evnode.node.aggregator=true \
  --evnode.rpc.address "0.0.0.0:7331" \
  --evnode.signer.passphrase $EVM_SIGNER_PASSPHRASE \
  --evnode.da.address $DA_ADDRESS \
  --evnode.da.auth_token $DA_AUTH_TOKEN \
  --evnode.da.namespace $DA_HEADER_NAMESPACE \
  --evnode.da.data_namespace $DA_DATA_NAMESPACE \
