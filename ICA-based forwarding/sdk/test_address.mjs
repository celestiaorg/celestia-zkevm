import { keccak256, encodeAbiParameters, concat, toHex, sha256 } from "viem";
import { toBech32 } from "@cosmjs/encoding";

const CELESTIA_ICA_FORWARD_V1_PREFIX = "CELESTIA_ICA_FORWARD_V1";
const CELESTIA_ICA_MODULE = "hl_ica";

// Test parameters - using realistic values
const tokenId = "0x0000000000000000000000000000000000000000000000000000000000000001";
const destDomain = 31338; // EVM Chain 2 domain
const destRecipient = "0x000000000000000000000000f39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

console.log("=== ICA Forwarding Address Computation ===\n");
console.log("Input Parameters:");
console.log("  Token ID:", tokenId);
console.log("  Destination Domain:", destDomain);
console.log("  Destination Recipient:", destRecipient);

// Step 1: Compute call digest
const callDigest = keccak256(
  encodeAbiParameters(
    [{ type: "bytes32" }, { type: "uint32" }, { type: "bytes32" }],
    [tokenId, destDomain, destRecipient]
  )
);
console.log("\nStep 1 - Call Digest:", callDigest);

// Step 2: Compute forwarding salt
const versionBytes = toHex(new TextEncoder().encode(CELESTIA_ICA_FORWARD_V1_PREFIX));
const salt = keccak256(concat([versionBytes, callDigest]));
console.log("Step 2 - Salt:", salt);

// Step 3: Derive Celestia address
const icaModuleBytes = toHex(new TextEncoder().encode(CELESTIA_ICA_MODULE));
const preimage = concat([icaModuleBytes, salt]);
const hash = sha256(preimage);
const addressBytes = hash.slice(0, 42); // 0x + 20 bytes (40 hex chars)

console.log("Step 3 - Address (hex):", addressBytes);

// Step 4: Convert to bech32
const bech32Addr = toBech32("celestia", Buffer.from(addressBytes.slice(2), 'hex'));
console.log("Step 4 - Address (bech32):", bech32Addr);

console.log("\n=== FORWARDING ADDRESS ===");
console.log(bech32Addr);
console.log("===========================\n");

// Generate encoded call for testing
console.log("Encoded forwarding call (for testing):");
const encodedCall = encodeAbiParameters(
  [{ type: "bytes32" }, { type: "uint32" }, { type: "bytes32" }, { type: "uint256" }],
  [tokenId, destDomain, destRecipient, 1000000000000000000n] // 1 token
);
console.log(encodedCall);

