#!/bin/bash
# Deploy ICA forwarding contracts and configure the InterchainAccountRouter
set -euo pipefail

# Configuration
export PRIVATE_KEY=${PRIVATE_KEY:-"0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a"}
export RPC_URL=${RPC_URL:-"http://localhost:8545"}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================="
echo "ICA Forwarding Contract Deployment"
echo "=========================================="
echo "RPC URL: $RPC_URL"
echo ""

# Check if foundry is installed
if ! command -v forge &> /dev/null; then
    echo "Error: Foundry (forge) is not installed"
    echo "Install with: curl -L https://foundry.paradigm.xyz | bash && foundryup"
    exit 1
fi

# Initialize foundry project if needed
if [ ! -d "lib/forge-std" ]; then
    echo "Installing forge-std..."
    forge install foundry-rs/forge-std --no-commit
fi

if [ ! -d "lib/openzeppelin-contracts" ]; then
    echo "Installing OpenZeppelin contracts..."
    forge install OpenZeppelin/openzeppelin-contracts@v4.9.0 --no-commit
fi

# Build contracts
echo ""
echo "Building contracts..."
forge build

# Deploy CelestiaICAHelper
echo ""
echo "Deploying CelestiaICAHelper..."
forge script script/DeployCelestiaICAHelper.s.sol:DeployCelestiaICAHelper \
    --rpc-url "$RPC_URL" \
    --broadcast \
    -vvv

# Enroll Celestia domain on InterchainAccountRouter
echo ""
echo "Enrolling Celestia domain on InterchainAccountRouter..."
forge script script/EnrollCelestiaDomain.s.sol:EnrollCelestiaDomain \
    --rpc-url "$RPC_URL" \
    --broadcast \
    -vvv

echo ""
echo "=========================================="
echo "Deployment Complete!"
echo "=========================================="

