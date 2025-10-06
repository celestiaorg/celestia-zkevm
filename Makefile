PROJECT_NAME=$(shell basename "$(PWD)")

## help: Get more info on make commands.
help: Makefile
	@echo " Choose a command run in "$(PROJECT_NAME)":"
	@sed -n 's/^##//p' $< | sort | column -t -s ':' | sed -e 's/^/ /'
.PHONY: help

## check-dependencies: Check if all dependencies are installed.
check-dependencies:
	@echo "--> Checking if all dependencies are installed"
	@if command -v cargo >/dev/null 2>&1; then \
		echo "✓ cargo is installed."; \
	else \
		echo "✗ Error: cargo is not installed. Please install Rust."; \
		exit 1; \
	fi
	@if command -v forge >/dev/null 2>&1; then \
		echo "✓ foundry is installed."; \
	else \
		echo "⚠ Warning: forge is not installed. query-balance target will not work."; \
	fi
	@if command -v cargo prove >/dev/null 2>&1; then \
		echo "✓ cargo prove is installed."; \
	else \
		echo "✗ Error: succinct is not installed. Please install SP1."; \
		exit 1; \
	fi
	@echo ""
	@echo "Note: transfer and transfer-back now use Rust binaries instead of Docker/cast"
	@echo "  - warp_transfer: Celestia → EVM rollup"
	@echo "  - transfer_back: EVM rollup → Celestia"
	@echo "All dependencies are installed."
.PHONY: check-dependencies

## build: Build Rust binaries for token transfers.
build:
	@echo "--> Building Rust transfer binaries"
	@cargo build --bin warp_transfer --release
	@cargo build --bin transfer_back --release
	@echo "✓ Binaries built:"
	@echo "  - target/release/warp_transfer"
	@echo "  - target/release/transfer_back"
.PHONY: build

## start: Start all Docker containers for the demo.
start:
	@echo "--> Starting all Docker containers"
	@docker compose up --detach
.PHONY: start

## stop: Stop all Docker containers and remove volumes.
stop:
	@echo "--> Stopping all Docker containers"
	@docker compose down -v
.PHONY: stop

## transfer: Transfer tokens from celestia-app to the EVM roll-up.
transfer:
	@echo "--> Transferring tokens from celestia-app to the EVM roll-up"
	@if [ -z "$$CELESTIA_PRIVATE_KEY" ]; then \
		echo "Error: CELESTIA_PRIVATE_KEY environment variable is not set."; \
		echo ""; \
		echo "Please source the .env file:"; \
		echo "  set -a && source .env && set +a"; \
		echo ""; \
		echo "Or set it manually:"; \
		echo "  export CELESTIA_PRIVATE_KEY=\"your_private_key_hex\""; \
		exit 1; \
	fi
	@CELESTIA_GRPC_ENDPOINT="http://localhost:9090" \
		cargo run --bin warp_transfer --release -- \
		--token-id "0x726f757465725f61707000000000000000000000000000010000000000000000" \
		--destination-domain 1234 \
		--recipient "0x000000000000000000000000aF9053bB6c4346381C77C2FeD279B17ABAfCDf4d" \
		--amount "10000010"
.PHONY: transfer

## transfer-back: Transfer tokens back from the EVM roll-up to celestia-app.
transfer-back:
	@echo "--> Transferring tokens back from the EVM roll-up to celestia-app"
	@EVM_RPC_URL="http://localhost:8545" \
		EVM_PRIVATE_KEY="0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a" \
		cargo run --bin transfer_back --release -- \
		--recipient "0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7" \
		--amount 1000
.PHONY: transfer-back

## transfer-back-loop: Loop transfer transactions back every second.
transfer-back-loop:
	@echo "--> Looping transfer transactions back every second"
	@while true; do \
		EVM_RPC_URL="http://localhost:8545" \
		EVM_PRIVATE_KEY="0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a" \
		cargo run --bin transfer_back --release -- \
		--recipient "0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7" \
		--amount 1000; \
		sleep 1; \
	done
.PHONY: transfer-back-loop

## query-balance: Query the balance of the receiver in the EVM roll-up.
query-balance:
	@echo "--> Querying the balance of the receiver on the EVM roll-up"
	@cast call 0x345a583028762De4d733852c9D4f419077093A48 \
  		"balanceOf(address)(uint256)" \
  		0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d \
  		--rpc-url http://localhost:8545
.PHONY: query-balance

## spamoor: Run spamoor transaction flooding against the EVM roll-up.
spamoor:
	@echo "--> Running spamoor transaction flooding daemon"
	@echo "Spamoor will be available on localhost:8080"
	@chmod +x scripts/run-spamoor.sh
	@scripts/run-spamoor.sh $(ARGS)
.PHONY: spamoor