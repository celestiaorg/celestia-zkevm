#!/bin/sh
set -e

cd /usr/bin

sleep 5

# Create default rollkit config if missing
if [ ! -f "$HOME/.evm-single/config/signer.json" ]; then
  ./evm-single init --rollkit.node.aggregator=true --rollkit.signer.passphrase $EVM_SIGNER_PASSPHRASE
fi

# Conditionally add --rollkit.da.address if ROLLKIT_DA_ADDRESS is set
da_flag=""
if [ -n "$DA_ADDRESS" ]; then
  da_flag="--rollkit.da.address $DA_ADDRESS"
fi

# Conditionally add --rollkit.da.auth_token if ROLLKIT_DA_AUTH_TOKEN is set
da_auth_token_flag=""
if [ -n "$DA_AUTH_TOKEN" ]; then
  da_auth_token_flag="--rollkit.da.auth_token $DA_AUTH_TOKEN"
fi

# Conditionally add --rollkit.da.header_namespace and --rollkit.da.data_namespace if set
da_header_namespace_flag=""
if [ -n "$DA_HEADER_NAMESPACE" ]; then
  da_header_namespace_flag="--rollkit.da.header_namespace $DA_HEADER_NAMESPACE"
fi

da_data_namespace_flag=""
if [ -n "$DA_DATA_NAMESPACE" ]; then
  da_data_namespace_flag="--rollkit.da.data_namespace $DA_DATA_NAMESPACE"
fi


exec ./evm-single start \
  --evm.jwt-secret $EVM_JWT_SECRET \
  --evm.genesis-hash $EVM_GENESIS_HASH \
  --evm.engine-url $EVM_ENGINE_URL \
  --evm.eth-url $EVM_ETH_URL \
  --rollkit.node.block_time $EVM_BLOCK_TIME \
  --rollkit.node.aggregator=true \
  --rollkit.rpc.address "0.0.0.0:7331" \
  --rollkit.signer.passphrase $EVM_SIGNER_PASSPHRASE \
  $da_flag \
  $da_auth_token_flag \
  $da_header_namespace_flag \
  $da_data_namespace_flag