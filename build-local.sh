docker build -t ghcr.io/celestiaorg/hyperlane-init:local -f hyperlane/Dockerfile .
cd devnet/forks/celestia-app
./build-local.sh
cd ../ev-node
./build-local.sh
