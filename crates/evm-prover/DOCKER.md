# EVM Prover Docker Setup

This directory contains the Docker configuration for the EVM Prover service.

## Files

- `Dockerfile` - Multi-stage Docker build for the evm-prover service
- `docker-entrypoint.sh` - Entrypoint script that handles initialization and startup
- `config.yaml` - Default configuration for the Docker environment
- `.dockerignore` - Excludes unnecessary files from the Docker build context

## Configuration

The default configuration in `config.yaml` is set up for the docker-compose environment:

- **gRPC Address**: `0.0.0.0:50051` (binds to all interfaces)
- **Celestia RPC**: `http://celestia-bridge:26658` (uses the bridge service)
- **EVM RPC**: `http://reth:8545` (uses the reth service)
- **Namespace**: Default namespace for the testnet
- **Public Key**: Default public key for the testnet

## Usage

The service is integrated into the main docker-compose.yml and will:

1. Build the evm-prover binary from source
2. Initialize the service configuration on first run
3. Start the gRPC server on port 50051
4. Connect to the celestia-bridge and reth services

## Health Check

The service includes a health check that verifies the gRPC server is responding on port 50051.

## Dependencies

The evm-prover service depends on:
- `ev-node-evm-single` - For getting the public key
- `celestia-bridge` - For Celestia RPC access

## Volumes

- `evm-prover-data` - Persistent storage for configuration and data
