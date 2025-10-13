docker run --rm -v celestia-app:/vol -v $(pwd):/backup busybox tar czf /backup/celestia-app.tar.gz -C /vol .
docker run --rm -v celestia-bridge:/vol -v $(pwd):/backup busybox tar czf /backup/celestia-bridge.tar.gz -C /vol .
docker run --rm -v hyperlane:/vol -v $(pwd):/backup busybox tar czf /backup/hyperlane.tar.gz -C /vol .
docker run --rm -v reth:/vol -v $(pwd):/backup busybox tar czf /backup/reth.tar.gz -C /vol .
docker run --rm -v evm-single-data:/vol -v $(pwd):/backup busybox tar czf /backup/evm-single-data.tar.gz -C /vol .

