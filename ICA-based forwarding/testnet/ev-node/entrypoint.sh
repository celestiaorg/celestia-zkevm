#!/bin/sh
set -e

cd /usr/bin

sleep 5

# Prepare passphrase and JWT files
PASSFILE=/tmp/evm_passphrase
echo -n "$EVM_SIGNER_PASSPHRASE" > "$PASSFILE"
JWTFILE=/tmp/jwt.hex
echo -n "$EVM_JWT_SECRET" > "$JWTFILE"

# Initialize config if missing
if [ ! -f "$HOME/.evm-single/config/evnode.yaml" ]; then
  ./evm-single init \
    --evnode.node.aggregator \
    --evnode.node.block_time "$EVM_BLOCK_TIME" \
    --evnode.rpc.address "0.0.0.0:7331" \
    --evnode.da.address "$DA_ADDRESS" \
    --evnode.da.auth_token "$DA_AUTH_TOKEN" \
    --evnode.da.namespace "$DA_NAMESPACE" \
    --evnode.signer.passphrase_file "$PASSFILE" \
    --evnode.signer.signer_path "$HOME/.evm-single/config" \
    --home "$HOME/.evm-single"
fi

exec ./evm-single start \
  --evm.engine-url "$EVM_ENGINE_URL" \
  --evm.eth-url "$EVM_ETH_URL" \
  --evm.genesis-hash "$EVM_GENESIS_HASH" \
  --evm.jwt-secret-file "$JWTFILE" \
  --evnode.node.aggregator \
  --evnode.node.block_time "$EVM_BLOCK_TIME" \
  --evnode.rpc.address "0.0.0.0:7331" \
  --evnode.da.address "$DA_ADDRESS" \
  --evnode.da.auth_token "$DA_AUTH_TOKEN" \
  --evnode.da.namespace "$DA_NAMESPACE" \
  --evnode.signer.passphrase_file "$PASSFILE" \
  --home "$HOME/.evm-single"

