#!/bin/bash
set -e

# Function to initialize the service
init_service() {
    echo "Initializing ev-prover service..."
    /app/ev-prover init
    
    # Create config directory if it doesn't exist
    mkdir -p "$HOME/.ev-prover/config"
}

# Function to start the service
start_service() {
    echo "Starting ev-prover gRPC server..."
    exec /app/ev-prover start
}

# Check if we should initialize first
if [ "$1" = "init" ]; then
    init_service
    exit 0
fi

# Check if config exists, if not initialize
if [ ! -f "$HOME/.ev-prover/config/config.yaml" ]; then
    echo "Config not found, initializing first..."
    init_service
fi

# Start the service
start_service
