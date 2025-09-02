#!/bin/bash
set -e

# Function to initialize the service
init_service() {
    echo "Initializing evm-prover service..."
    /app/evm-prover init
    
    # Copy default config if it doesn't exist
    if [ ! -f "$HOME/.evm-prover/config/config.yaml" ]; then
        echo "Copying default configuration..."
        mkdir -p "$HOME/.evm-prover/config"
        cp /app/config.yaml "$HOME/.evm-prover/config/config.yaml"
    fi
}

# Function to start the service
start_service() {
    echo "Starting evm-prover gRPC server..."
    exec /app/evm-prover start
}

# Check if we should initialize first
if [ "$1" = "init" ]; then
    init_service
    exit 0
fi

# Check if config exists, if not initialize
if [ ! -f "$HOME/.evm-prover/config/config.yaml" ]; then
    echo "Config not found, initializing first..."
    init_service
fi

# Start the service
start_service
