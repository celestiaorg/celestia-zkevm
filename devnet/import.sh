# Stop the stack before restoring
docker compose down

# Remove old volumes
docker volume rm celestia-app celestia-bridge hyperlane reth evm-single-data

# Recreate empty volumes
docker volume create celestia-app
docker volume create celestia-bridge
docker volume create hyperlane
docker volume create reth
docker volume create evm-single-data

# Restore Celestia App
docker run --rm -v celestia-app:/vol -v $(pwd):/backup busybox tar xzf /backup/celestia-app.tar.gz -C /vol

# Restore Celestia Bridge
docker run --rm -v celestia-bridge:/vol -v $(pwd):/backup busybox tar xzf /backup/celestia-bridge.tar.gz -C /vol

# Restore Hyperlane
docker run --rm -v hyperlane:/vol -v $(pwd):/backup busybox tar xzf /backup/hyperlane.tar.gz -C /vol

# Restore Reth (EVM node)
docker run --rm -v reth:/vol -v $(pwd):/backup busybox tar xzf /backup/reth.tar.gz -C /vol

# Restore zkEVM single node data
docker run --rm -v evm-single-data:/vol -v $(pwd):/backup busybox tar xzf /backup/evm-single-data.tar.gz -C /vol

# Bring the stack back up
docker compose up -d

