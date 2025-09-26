#!/bin/sh
set -e

cd /usr/bin

sleep 5

# Create default evnode config if missing
# TODO: The --evnode.signer.path flag is not respected: https://github.com/evstack/ev-node/issues/2603
if [ ! -f "$HOME/.evm-single/config/signer.json" ]; then
  ./evm-single init --evnode.node.aggregator=true --evnode.signer.path $EVM_SIGNER_PATH --evnode.signer.passphrase $EVM_SIGNER_PASSPHRASE
fi

# Conditionally add --evnode.da.address if DA_ADDRESS is set
da_flag=""
if [ -n "$DA_ADDRESS" ]; then
  da_flag="--evnode.da.address $DA_ADDRESS"
fi

# Conditionally add --evnode.da.auth_token if DA_AUTH_TOKEN is set
da_auth_token_flag=""
if [ -n "$DA_AUTH_TOKEN" ]; then
  da_auth_token_flag="--evnode.da.auth_token $DA_AUTH_TOKEN"
fi

# Conditionally add --evnode.da.header_namespace and --evnode.da.data_namespace if set
da_header_namespace_flag=""
if [ -n "$DA_HEADER_NAMESPACE" ]; then
  da_header_namespace_flag="--evnode.da.namespace $DA_HEADER_NAMESPACE"
fi

da_data_namespace_flag=""
if [ -n "$DA_DATA_NAMESPACE" ]; then
  da_data_namespace_flag="--evnode.da.data_namespace $DA_DATA_NAMESPACE"
fi

exec ./evm-single start \
  --evm.jwt-secret $EVM_JWT_SECRET \
  --evm.genesis-hash $EVM_GENESIS_HASH \
  --evm.engine-url $EVM_ENGINE_URL \
  --evm.eth-url $EVM_ETH_URL \
  --evnode.node.block_time $EVM_BLOCK_TIME \
  --evnode.node.aggregator=true \
  --evnode.rpc.address "0.0.0.0:7331" \
  --evnode.signer.path $EVM_SIGNER_PATH \
  --evnode.signer.passphrase $EVM_SIGNER_PASSPHRASE \
  $da_flag \
  $da_auth_token_flag \
  $da_header_namespace_flag \
  $da_data_namespace_flag