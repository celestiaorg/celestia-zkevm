PROJECT_NAME=$(shell basename "$(PWD)")

# Pinned celestia-app image: celestia-app jonas/root-history-hyperlane @ 71d90a17 (x/zkism root history,
# tag 71d90a1 of celestiaorg/celestia-app-standalone). Groth16 verifier patched for SP1 v6 proofs.
# Must match the digest referenced in docker-compose.yml.
CELESTIA_IMAGE ?= docker.io/celestiaorg/celestia-app-standalone@sha256:7c3118993893b874da537417c748452bcc00d471f488eaac0cb0221694bf2504

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
  		--network celestia-zkevm_celestia-zkevm-net \
  		--volume celestia-zkevm_celestia-app:/home/celestia/.celestia-app \
  		$(CELESTIA_IMAGE) \
  		tx warp transfer 0x726f757465725f61707000000000000000000000000000010000000000000000 1234 0x000000000000000000000000aF9053bB6c4346381C77C2FeD279B17ABAfCDf4d "10000000" \
  		--from default --fees 1000utia --gas auto --max-hyperlane-fee 36400utia --node http://celestia-validator:26657 --yes
.PHONY: transfer

## transfer-back: Transfer tokens back from the EVM roll-up to celestia-app.
transfer-back:
	@echo "--> Transferring tokens back from the EVM roll-up to celestia-app"
	@cast send 0xdCEa98814d56e04Dd8b316C79D62eFD49Bc34840 \
  		"transferRemote(uint32, bytes32, uint256)(bytes32)" \
  		69420 0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7 1000 \
		--private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
  		--rpc-url http://localhost:8545
.PHONY: transfer-back

## transfer-back-loop: Loop transfer transactions back every second.
transfer-back-loop:
	@echo "--> Looping transfer transactions back every second"
	@while true; do \
		cast send 0xdCEa98814d56e04Dd8b316C79D62eFD49Bc34840 \
  		"transferRemote(uint32, bytes32, uint256)(bytes32)" \
  		69420 0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7 1000 \
		--private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
  		--rpc-url http://localhost:8545 \
		sleep 1; \
	done
.PHONY: transfer-back-loop

## query-balance: Query the balance of the receiver in the EVM roll-up.
query-balance:
	@echo "--> Querying the balance of the receiver on the EVM roll-up"
	@cast call 0xdCEa98814d56e04Dd8b316C79D62eFD49Bc34840 \
  		"balanceOf(address)(uint256)" \
  		0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d \
  		--rpc-url http://localhost:8545
.PHONY: query-balance

## spamoor: Run spamoor transaction flooding against the EVM roll-up.
spamoor:
	@echo "--> Running spamoor transaction flooding daemon"
	@echo "Spamoor will be available on localhost:8080"
	@chmod +x testdata/spamoor/run-spamoor.sh
	@testdata/spamoor/run-spamoor.sh $(ARGS)
.PHONY: spamoor

docker-build-hyperlane:
	@echo "--> Building hyperlane-init image"
	@docker build --platform linux/amd64 -t ghcr.io/celestiaorg/hyperlane-init:local -f testnet/hyperlane/Dockerfile .
.PHONY: docker-build-hyperlane

deploy-ism:
	@echo "--> Deploying ISM"
	@RUST_LOG="ev_prover=info" cargo run -p ev-prover create-ism
.PHONY: deploy-ism

deploy-ism-tee:
	@echo "--> Deploying ISM"
	@RUST_LOG="ev_prover=info" cargo run --features tee_mode -p ev-prover create-ism
.PHONY: deploy-ism-tee

update-ism:
	@echo "--> Updating ISM"
	@RUST_LOG="ev_prover=info" cargo run -p ev-prover set-token-ism 0x726f757465725f69736d000000000000000000000000002a0000000000000002 0x726f757465725f61707000000000000000000000000000010000000000000000
.PHONY: update-ism
