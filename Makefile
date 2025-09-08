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
		echo "cargo is installed."; \
	else \
		echo "Error: cargo is not installed. Please install Rust."; \
		exit 1; \
	fi
	@if command -v forge >/dev/null 2>&1; then \
		echo "foundry is installed."; \
	else \
		echo "Error: forge is not installed. Please install Foundry."; \
		exit 1; \
	fi
	@if command -v cargo prove >/dev/null 2>&1; then \
		echo "cargo prove is installed."; \
	else \
		echo "Error: succinct is not installed. Please install SP1."; \
		exit 1; \
	fi
	@echo "All dependencies are installed."
.PHONY: check-dependencies

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
	@docker run --rm \
  		--network celestia-zkevm-hl-testnet_celestia-zkevm-net \
  		--volume celestia-zkevm-hl-testnet_celestia-app:/home/celestia/.celestia-app \
  		ghcr.io/celestiaorg/celestia-app-standalone:feature-zk-execution-ism \
  		tx warp transfer 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000aF9053bB6c4346381C77C2FeD279B17ABAfCDf4d "1000000" \
  		--from default --fees 400utia --max-hyperlane-fee 100utia --node http://celestia-validator:26657 --yes
.PHONY: transfer

## transfer-back: Transfer tokens back from the EVM roll-up to celestia-app (nonces 28..78).
transfer-back:
	@for nonce in $$(seq 89 135); do \
		( \
			echo ">> sending tx with nonce=$$nonce"; \
			cast send 0xa7578551baE89a96C3365b93493AD2D4EBcbAe97 \
				"transferRemote(uint32, bytes32, uint256)(bytes32)" \
				69420 0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7 1000 \
				--private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
				--rpc-url http://localhost:8545 \
				--nonce $$nonce \
				--async \
		) & \
	done; \
	wait
.PHONY: transfer-back


## query-balance: Query the balance of the receiver in the EVM roll-up.
query-balance:
	@echo "--> Querying the balance of the receiver on the EVM roll-up"
	@cast call 0xa7578551baE89a96C3365b93493AD2D4EBcbAe97 \
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
