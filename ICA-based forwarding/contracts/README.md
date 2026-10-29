# ICA Forwarding Contracts

This directory contains Solidity contracts for the ICA-based forwarding pattern.

## Contracts

### CelestiaICAHelper.sol

A helper contract that provides utilities for computing Celestia ICA forwarding addresses and encoding calls. Key functions:

- `computeForwardingAddress(tokenId, destDomain, destRecipient)` - Computes the deterministic forwarding address on Celestia
- `encodeForwardingCall(tokenId, destDomain, destRecipient, amount)` - Encodes the call data for forwarding
- `validateForwardingAddress(...)` - Validates that an address matches the expected intent

### Address Derivation

The forwarding address is derived as:

```
callDigest = keccak256(abi.encode(tokenId, destDomain, destRecipient))
salt = keccak256(abi.encodePacked("CELESTIA_ICA_FORWARD_V1", callDigest))
forwardAddr = deriveModuleAccount(celestiaICAModule, salt)
```

This binds the address cryptographically to the forwarding intent, ensuring funds can only be forwarded to the committed destination.

## Deployment

### Prerequisites

- [Foundry](https://book.getfoundry.sh/getting-started/installation) installed
- Local chains running (EVM Chain 1 and Celestia)

### Deploy

```bash
# Set your private key (default uses the test key from docker-compose)
export PRIVATE_KEY=0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a

# Deploy contracts and configure ICA router
./deploy.sh
```

### Manual Deployment

```bash
# Build contracts
forge build

# Deploy CelestiaICAHelper
forge script script/DeployCelestiaICAHelper.s.sol --rpc-url http://localhost:8545 --broadcast

# Enroll Celestia domain on InterchainAccountRouter
forge script script/EnrollCelestiaDomain.s.sol --rpc-url http://localhost:8545 --broadcast
```

## Existing Deployments

### EVM Chain 1 (rethlocal)

| Contract | Address |
|----------|---------|
| InterchainAccountRouter | `0x4dc4E8bf5D0390C95Af9AFEb1e9c9927c4dB83e7` |
| InterchainAccountISM | `0x9F098AE0AC3B7F75F0B3126f471E5F592b47F300` |
| Mailbox | `0xb1c938F5BA4B3593377F399e12175e8db0C787Ff` |
| Warp Route | `0x345a583028762De4d733852c9D4f419077093A48` |

### Celestia

| Component | ID |
|-----------|-----|
| Domain ID | 69420 |
| Mailbox ID | `0x68797065726c616e65...` |
| ISM ID | `0x726f757465725f69736d...` |

## Usage Example

```solidity
// Get forwarding address for an intent
CelestiaICAHelper helper = CelestiaICAHelper(helperAddress);

bytes32 tokenId = 0x...; // Celestia warp token ID
uint32 destDomain = 5678; // EVM Chain 2 domain
bytes32 destRecipient = bytes32(uint256(uint160(recipientAddress)));

// Compute the forwarding address
bytes32 forwardAddr = helper.computeForwardingAddress(tokenId, destDomain, destRecipient);

// User sends tokens to forwardAddr via warp route
// Then triggers forwarding via ICA call
```

