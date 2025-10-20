# Native TIA Minter for Hyperlane

This implements a Hyperlane Warp Route that mints **native TIA** (gas token) on Eden when bridging from Celestia.

## What's Inside

### Contracts

1. **`HypNativeMinter.sol`** - Main contract that receives Hyperlane messages and mints native tokens
2. **`MockNativeMinter.sol`** - Mock precompile for testing (simulates the real native minting precompile)
3. **`INativeMinter.sol`** - Interface for the native minting precompile

### How It Works

```
Celestia (TIA) ──[bridge]──> Eden (Native ETH for gas)
```

1. User burns TIA on Celestia
2. Hyperlane message sent to Eden
3. HypNativeMinter receives message
4. Calls native minting precompile
5. Recipient gets native ETH (usable for gas!)

### Key Features

- ✅ Mints **native tokens** (not ERC20)
- ✅ Decimal conversion (6 decimals → 18 decimals)
- ✅ Router enrollment for cross-chain routing
- ✅ Admin controls for security
- ✅ Mock precompile for testing without ev-reth changes

## Run Integration Tests

```bash
# From contracts directory
forge test -vv

# Or run specific test
forge test -vv --match-contract HypNativeMinterIntegration
```

**What you get:**
- 13 passing tests
- Full bridge flow simulation (Celestia → Eden → back)
- Decimal conversion verification
- Router enrollment tests
- Native minting validation

## Files

| File | Purpose |
|------|---------|
| `src/HypNativeMinter.sol` | Production native minter contract |
| `src/MockNativeMinter.sol` | Mock precompile for testing |
| `src/INativeMinter.sol` | Precompile interface |
| `test/HypNativeMinterIntegration.t.sol` | Full integration tests (13 tests) |
| `test/HypNativeMinter.t.sol` | Unit tests (4 tests) |
| `script/DeployWithMockPrecompile.s.sol` | Deploy script |

## Switch to Real Precompile

When the real precompile is ready:

1. Update `HypNativeMinter` constructor:
   ```solidity
   // Change from:
   nativeMinter = INativeMinter(mockPrecompile);

   // To:
   nativeMinter = INativeMinter(0x0000000000000000000000000000000000000800);
   ```

2. Redeploy
3. Done!

Takes ~5 minutes.
