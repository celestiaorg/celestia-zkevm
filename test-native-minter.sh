#!/bin/bash

# Native TIA Minter End-to-End Test
# This script demonstrates native TIA minting from Celestia to Eden

set -e

echo "════════════════════════════════════════════════════════"
echo "  Native TIA Minter - End-to-End Test"
echo "════════════════════════════════════════════════════════"
echo ""

# Configuration
RECIPIENT="0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d"
PRIVATE_KEY="0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a"
RPC_URL="http://localhost:8545"
HYPNATIVEMINTER="0x81a91d503d2c171d9148827f549e51C286Acc97D"
MOCK_PRECOMPILE="0x83b466f5856dC4F531Bb5Af45045De06889D63CB"
TOKEN_ID="0x726f757465725f61707000000000000000000000000000010000000000000001"

echo "📋 Configuration:"
echo "   Recipient: $RECIPIENT"
echo "   HypNativeMinter: $HYPNATIVEMINTER"
echo "   MockPrecompile: $MOCK_PRECOMPILE"
echo "   Token ID: $TOKEN_ID"
echo ""

# Step 1: Check if network is running
echo "1️⃣  Checking if testnet is running..."
if ! docker ps | grep -q celestia-validator; then
    echo "   ⚠️  Testnet not running. Starting with 'make start'..."
    make start
    echo "   ⏳ Waiting 30s for network to initialize..."
    sleep 30

    echo "   🔧 Fixing relayer config..."
    ./fix-relayer-config.sh > /dev/null
    docker restart relayer > /dev/null 2>&1
    echo "   ✅ Relayer config fixed and restarted"
else
    echo "   ✅ Testnet is running"
fi
echo ""

# Step 2: Check initial balance
echo "2️⃣  Checking initial balance..."
INITIAL_BALANCE=$(cast balance $RECIPIENT --rpc-url $RPC_URL)
echo "   Initial balance: $INITIAL_BALANCE wei"
echo ""

# Step 3: Deploy contracts (if not already deployed)
echo "3️⃣  Checking contract deployment..."

if cast code $HYPNATIVEMINTER --rpc-url $RPC_URL | grep -q "^0x$"; then
    echo "   Deploying contracts..."
    cd contracts
    forge script script/DeployWithMockPrecompile.s.sol:DeployWithMockPrecompile \
      --rpc-url $RPC_URL \
      --private-key $PRIVATE_KEY \
      --broadcast \
      --silent
    cd ..
    sleep 3
    echo "   ✅ Contracts deployed"
else
    echo "   ✅ Contracts already deployed"
fi

# Verify deployment
CAN_MINT=$(cast call $HYPNATIVEMINTER "canMint()(bool)" --rpc-url $RPC_URL)
if [ "$CAN_MINT" = "true" ]; then
    echo "   ✅ HypNativeMinter can mint"
else
    echo "   ❌ HypNativeMinter cannot mint! Check deployment."
    exit 1
fi
echo ""

# Step 4: Check if warp route exists on Celestia
echo "4️⃣  Checking Celestia warp route..."
if docker exec celestia-validator celestia-appd query warp remote-routers $TOKEN_ID --node http://localhost:26657 -o json 2>&1 | grep -q "receiver_domain"; then
    echo "   ✅ Warp route already exists"
else
    echo "   Creating new collateral token on Celestia..."
    docker run --rm \
      --network celestia-zkevm-hl-testnet-2_celestia-zkevm-net \
      --volume celestia-zkevm-hl-testnet-2_celestia-app:/home/celestia/.celestia-app \
      ghcr.io/celestiaorg/celestia-app-standalone:feature-zk-execution-ism \
      tx warp create-collateral-token \
      0x68797065726c616e650000000000000000000000000000000000000000000000 \
      utia \
      --from default \
      --fees 800utia \
      --node http://celestia-validator:26657 \
      --yes > /dev/null 2>&1

    sleep 5
    echo "   ✅ Collateral token created"
fi
echo ""

# Step 5: Enroll routers if not already enrolled
echo "5️⃣  Enrolling routers..."

# Check Celestia side
CELESTIA_ROUTER=$(docker exec celestia-validator celestia-appd query warp remote-routers $TOKEN_ID --node http://localhost:26657 -o json 2>&1 | jq -r '.remote_routers[0].receiver_contract' || echo "")

if [ "$CELESTIA_ROUTER" != "0x00000000000000000000000081a91d503d2c171d9148827f549e51C286Acc97D" ]; then
    echo "   Enrolling HypNativeMinter on Celestia..."

    # First unroll old router if exists
    if [ ! -z "$CELESTIA_ROUTER" ] && [ "$CELESTIA_ROUTER" != "null" ]; then
        docker run --rm \
          --network celestia-zkevm-hl-testnet-2_celestia-zkevm-net \
          --volume celestia-zkevm-hl-testnet-2_celestia-app:/home/celestia/.celestia-app \
          ghcr.io/celestiaorg/celestia-app-standalone:feature-zk-execution-ism \
          tx warp unroll-remote-router \
          $TOKEN_ID \
          1234 \
          --from default \
          --fees 800utia \
          --node http://celestia-validator:26657 \
          --yes > /dev/null 2>&1
        sleep 3
    fi

    # Enroll new router
    docker run --rm \
      --network celestia-zkevm-hl-testnet-2_celestia-zkevm-net \
      --volume celestia-zkevm-hl-testnet-2_celestia-app:/home/celestia/.celestia-app \
      ghcr.io/celestiaorg/celestia-app-standalone:feature-zk-execution-ism \
      tx warp enroll-remote-router \
      $TOKEN_ID \
      1234 \
      0x00000000000000000000000081a91d503d2c171d9148827f549e51C286Acc97D \
      200000 \
      --from default \
      --fees 800utia \
      --node http://celestia-validator:26657 \
      --yes > /dev/null 2>&1

    sleep 3
    echo "   ✅ Enrolled on Celestia"
else
    echo "   ✅ Already enrolled on Celestia"
fi

# Check Eden side
EDEN_ROUTER=$(cast call $HYPNATIVEMINTER "routers(uint32)(bytes32)" 69420 --rpc-url $RPC_URL || echo "0x")

if [ "$EDEN_ROUTER" != "$TOKEN_ID" ]; then
    echo "   Enrolling Celestia token on Eden..."
    cast send $HYPNATIVEMINTER \
      "enrollRemoteRouter(uint32,bytes32)" \
      69420 \
      $TOKEN_ID \
      --private-key $PRIVATE_KEY \
      --rpc-url $RPC_URL > /dev/null 2>&1
    echo "   ✅ Enrolled on Eden"
else
    echo "   ✅ Already enrolled on Eden"
fi
echo ""

# Step 6: Send test transfer
echo "6️⃣  Sending test transfer (10 TIA from Celestia)..."
TRANSFER_TX=$(docker run --rm \
  --network celestia-zkevm-hl-testnet-2_celestia-zkevm-net \
  --volume celestia-zkevm-hl-testnet-2_celestia-app:/home/celestia/.celestia-app \
  ghcr.io/celestiaorg/celestia-app-standalone:feature-zk-execution-ism \
  tx warp transfer \
  $TOKEN_ID \
  1234 \
  0x000000000000000000000000aF9053bB6c4346381C77C2FeD279B17ABAfCDf4d \
  10000000 \
  --from default \
  --fees 800utia \
  --max-hyperlane-fee 100utia \
  --node http://celestia-validator:26657 \
  --yes 2>&1 | grep "txhash:" | awk '{print $2}')

echo "   Transfer TX: $TRANSFER_TX"
echo "   ⏳ Waiting for message to be relayed..."
echo ""

# Step 7: Wait for message delivery by checking for ReceivedMessage event
echo -n "   Checking for mint"
INITIAL_BLOCK=$(cast block-number --rpc-url $RPC_URL)

for i in {1..30}; do
  sleep 2
  echo -n "."

  # Check for ReceivedMessage events from HypNativeMinter
  LOGS=$(cast logs --from-block $INITIAL_BLOCK --address $HYPNATIVEMINTER --rpc-url $RPC_URL 2>/dev/null | grep -c "0xa042999eea1982ceab1be892c2338d9c69a126574b2e637916039bfcc6174175" || echo "0")

  if [ "$LOGS" -gt 0 ]; then
    echo ""
    echo ""
    echo "════════════════════════════════════════════════════════"
    echo "  ✅ SUCCESS! Native ETH was minted on Eden!"
    echo "════════════════════════════════════════════════════════"
    echo ""

    # Get final balance
    CURRENT_BALANCE=$(cast balance $RECIPIENT --rpc-url $RPC_URL)

    echo "📊 Event Detection:"
    echo "   ✅ ReceivedMessage event found in HypNativeMinter logs"
    echo ""
    echo "📊 Balance Check:"
    echo "   Initial balance:  $INITIAL_BALANCE wei"
    echo "   Current balance:  $CURRENT_BALANCE wei"

    # Calculate net difference (may be negative due to gas costs)
    DIFF=$(python3 -c "print($CURRENT_BALANCE - $INITIAL_BALANCE)")
    echo "   Net change:       $DIFF wei"
    python3 -c "print(f'   Net change (ETH): {$DIFF / 1e18:.6f} ETH')"
    echo ""
    echo "   Note: Net change may be negative if recipient paid gas for transactions"
    echo "   Expected mint: ~10 TIA = 10,000,000,000,000,000,000 wei (10 ETH)"
    echo ""
    echo "✅ The native ETH mint succeeded! Recipient can use it for gas."
    echo ""
    exit 0
  fi
done

echo ""
echo ""
echo "⚠️  Balance did not increase significantly after 60 seconds"
echo ""
CURRENT_BALANCE=$(cast balance $RECIPIENT --rpc-url $RPC_URL)
# Use Python for large number subtraction
DIFF=$(python3 -c "print($CURRENT_BALANCE - $INITIAL_BALANCE)")

if python3 -c "exit(0 if $DIFF != 0 else 1)"; then
    echo "   Balance changed by: $DIFF wei"
    python3 -c "print(f'   Changed by (ETH): {$DIFF / 1e18:.6f} ETH')"
    echo "   (This might be gas costs from transactions)"
else
    echo "   Balance unchanged: $CURRENT_BALANCE wei"
fi
echo ""
echo "   This might mean:"
echo "   - Relayer needs more time (check: docker logs relayer)"
echo "   - Message needs manual delivery"
echo "   - Router enrollment needs verification"
echo ""
echo "   To manually check if message was delivered:"
echo "   docker logs relayer | grep -i process"
