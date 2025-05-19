#!/bin/bash

set -euo pipefail

export HYP_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a

echo "Using Hyperlane registry:"
hyperlane registry list --registry ./registry

echo "Deploying Hyperlane core EVM contracts..."
hyperlane core deploy --chain rethlocal --registry ./registry --yes

echo "Deploying Hyperlane warp synthetic token EVM contracts..."
hyperlane warp deploy --config ./configs/warp-config.yaml --registry ./registry --yes

echo "Deploying Hyperlane on cosmosnative..."
hyp deploy celestia-validator:9090
