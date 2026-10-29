# ICA Forwarding SDK

TypeScript SDK for ICA-based token forwarding through Celestia.

## Installation

```bash
npm install
# or
yarn install
```

## Usage

### Web UI (Recommended for Testing)

The SDK includes a standalone web UI for computing forwarding addresses interactively.

#### Quick Start

1. **Option A: Open directly in browser**
   ```bash
   # Simply open the HTML file in your browser
   open ui/index.html
   ```

2. **Option B: Serve locally with Python**
   ```bash
   cd ui
   python3 -m http.server 3000
   # Then open http://localhost:3000 in your browser
   ```

3. **Option C: Use the serve script**
   ```bash
   ./ui/serve.sh
   ```

#### UI Features

- **Compute Address Tab**: Enter token ID, destination domain, and recipient to compute the deterministic forwarding address on Celestia
- **Encode Call Tab**: Generate the ICA call data for triggering a forwarding operation
- **Real-time validation**: See call digest, salt, and address derivation steps
- **Copy-to-clipboard**: Easily copy computed addresses and encoded data

#### Example Inputs

```
Token ID:        0x726f757465725f61707000000000000000000000000000010000000000000000
Dest Domain:     5678
Dest Recipient:  0x000000000000000000000000f39Fd6e51aad88F6F4ce6aB8827279cffFb92266
```

The UI will output the Celestia forwarding address (e.g., `celestia1...`) along with the intermediate computation values for verification.

---

### As a Library

```typescript
import { 
  computeForwardingAddress, 
  prepareForwarding,
  addressToBytes32,
  CHAINS,
} from "@celestia-ica/forwarding-sdk";

// Compute a forwarding address
const intent = {
  tokenId: CHAINS.celestia.warpTokenId,
  destDomain: 5678, // EVM Chain 2
  destRecipient: addressToBytes32("0x1234..."), // Final recipient
};

const result = computeForwardingAddress(intent);
console.log("Send tokens to:", result.celestiaAddress);

// Prepare a complete forwarding transaction
const { forwardingAddress, icaCall, icaMessage } = prepareForwarding(
  CHAINS.celestia.warpTokenId,
  5678,
  "0x1234...",
  1000000n // amount
);
```

### CLI Commands

```bash
# Compute a forwarding address
npx ts-node src/cli/compute-address.ts \
  --token 0x726f757465725f61707000000000000000000000000000010000000000000000 \
  --dest-domain 5678 \
  --recipient 0x1234567890123456789012345678901234567890

# Execute a forwarding transaction
npx ts-node src/cli/execute-forward.ts \
  --token 0x... \
  --dest-domain 5678 \
  --recipient 0x... \
  --amount 1000000 \
  --private-key 0x...
```

### Using the CLI

After building:

```bash
npm run build

# Then use the CLI
./dist/cli/index.js compute-address --token 0x... --dest-domain 5678 --recipient 0x...
./dist/cli/index.js validate-address --address celestia1... --token 0x... --dest-domain 5678 --recipient 0x...
./dist/cli/index.js encode-call --token 0x... --dest-domain 5678 --recipient 0x... --amount 1000000
```

## API Reference

### Address Computation

#### `computeForwardingAddress(intent)`

Computes the deterministic forwarding address on Celestia for a given intent.

```typescript
const result = computeForwardingAddress({
  tokenId: "0x...",      // Warp token ID on Celestia
  destDomain: 5678,      // Final destination domain
  destRecipient: "0x...", // Final recipient (bytes32)
});

// Returns:
// {
//   celestiaAddress: "celestia1...",
//   addressBytes32: "0x...",
//   salt: "0x...",
//   callDigest: "0x...",
// }
```

#### `validateForwardingAddress(address, intent)`

Validates that an address matches a forwarding intent.

```typescript
const isValid = validateForwardingAddress("celestia1...", intent);
```

#### `addressToBytes32(address)`

Converts an EVM address to bytes32 (zero-padded).

```typescript
const bytes32 = addressToBytes32("0x1234...");
// "0x0000000000000000000000001234..."
```

### Encoding

#### `buildForwardingCall(warpModuleAddress, intent, amount)`

Builds an ICA Call struct for forwarding.

```typescript
const call = buildForwardingCall(
  CHAINS.celestia.warpTokenId,
  intent,
  1000000n
);
// { to: "0x...", value: 0n, data: "0x..." }
```

#### `encodeICACallsMessage(owner, ism, calls, userSalt?)`

Encodes a complete ICA CALLS message.

```typescript
const message = encodeICACallsMessage(
  ownerBytes32,
  ismBytes32,
  [call1, call2],
  salt
);
```

### High-Level Helpers

#### `prepareForwarding(tokenId, destDomain, destRecipient, amount)`

Prepares all data needed for a forwarding transaction.

```typescript
const { forwardingAddress, icaCall, icaMessage } = prepareForwarding(
  tokenId,
  destDomain,
  recipient,
  amount
);
```

## Address Derivation

The forwarding address is derived deterministically from the forwarding intent:

```
callDigest = keccak256(abi.encode(tokenId, destDomain, destRecipient))
salt = keccak256(abi.encodePacked("CELESTIA_ICA_FORWARD_V1", callDigest))
forwardAddr = deriveCelestiaModuleAccount(icaModule, salt)
```

This ensures:
- The address is cryptographically bound to the intent
- Funds can only be forwarded to the committed destination
- Anyone can compute the address off-chain
- The forwarding is permissionless (anyone can trigger execution)

## Constants

The SDK includes pre-configured constants for the local testnet:

```typescript
import { CHAINS, GAS_LIMITS, FORWARD_VERSION } from "@celestia-ica/forwarding-sdk";

CHAINS.rethlocal    // EVM Chain 1 config
CHAINS.celestia     // Celestia config  
CHAINS.rethlocal2   // EVM Chain 2 config

GAS_LIMITS.ICA_CALL       // 500000n
GAS_LIMITS.WARP_TRANSFER  // 200000n
```

