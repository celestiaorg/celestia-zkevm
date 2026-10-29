/**
 * Address derivation functions for ICA-based forwarding
 */

import { keccak256, encodeAbiParameters, parseAbiParameters, concat, toHex, hexToBytes, bytesToHex } from "viem";
import { sha256 } from "@cosmjs/crypto";
import { toBech32 } from "@cosmjs/encoding";
import { ForwardingIntent, ForwardingAddressResult, ICAAddressParams } from "./types";
import { FORWARD_VERSION, EMPTY_SALT, CHAINS } from "./constants";

/**
 * Computes the forwarding salt from intent parameters
 * @param intent The forwarding intent
 * @returns The salt as bytes32 hex string
 */
export function computeForwardingSalt(intent: ForwardingIntent): string {
  // Compute call digest: keccak256(abi.encode(tokenId, destDomain, destRecipient))
  const callDigest = keccak256(
    encodeAbiParameters(
      parseAbiParameters("bytes32, uint32, bytes32"),
      [intent.tokenId as `0x${string}`, intent.destDomain, intent.destRecipient as `0x${string}`]
    )
  );

  // Compute salt: keccak256(abi.encodePacked(FORWARD_VERSION, callDigest))
  const versionBytes = toHex(new TextEncoder().encode(FORWARD_VERSION));
  const salt = keccak256(concat([versionBytes as `0x${string}`, callDigest]));

  return salt;
}

/**
 * Computes the call digest for a forwarding intent
 * @param intent The forwarding intent
 * @returns The call digest as bytes32 hex string
 */
export function computeCallDigest(intent: ForwardingIntent): string {
  return keccak256(
    encodeAbiParameters(
      parseAbiParameters("bytes32, uint32, bytes32"),
      [intent.tokenId as `0x${string}`, intent.destDomain, intent.destRecipient as `0x${string}`]
    )
  );
}

/**
 * Derives a Celestia module account address from a module address and salt
 * 
 * Cosmos SDK module account derivation:
 * address = sha256(moduleAddress || salt)[:20]
 * 
 * @param moduleAddress The module address (bytes32 hex string)
 * @param salt The derivation salt (bytes32 hex string)
 * @returns The derived address as bytes (20 bytes)
 */
export function deriveCelestiaModuleAddress(moduleAddress: string, salt: string): Uint8Array {
  const moduleBytes = hexToBytes(moduleAddress as `0x${string}`);
  const saltBytes = hexToBytes(salt as `0x${string}`);
  
  // Concatenate module address and salt
  const preimage = new Uint8Array(moduleBytes.length + saltBytes.length);
  preimage.set(moduleBytes, 0);
  preimage.set(saltBytes, moduleBytes.length);
  
  // SHA256 hash and take first 20 bytes (Cosmos address length)
  const hash = sha256(preimage);
  return hash.slice(0, 20);
}

/**
 * Computes the ICA address on Celestia using the same derivation as InterchainAccountRouter
 * 
 * The ICA address is derived from:
 * - origin domain
 * - owner address
 * - router address
 * - ISM address
 * - user salt
 * 
 * @param params The ICA address parameters
 * @returns The derived address as bytes32 hex string
 */
export function computeICAAddress(params: ICAAddressParams): string {
  const userSalt = params.userSalt || EMPTY_SALT;
  
  // Match InterchainAccountRouter._getSalt()
  // salt = keccak256(abi.encodePacked(_origin, _owner, _router, _ism, _userSalt))
  const salt = keccak256(
    encodeAbiParameters(
      parseAbiParameters("uint32, bytes32, bytes32, bytes32, bytes32"),
      [
        params.origin,
        params.owner as `0x${string}`,
        params.router as `0x${string}`,
        params.ism as `0x${string}`,
        userSalt as `0x${string}`,
      ]
    )
  );

  return salt;
}

/**
 * Computes the forwarding address on Celestia for a given intent
 * 
 * @param intent The forwarding intent
 * @param celestiaConfig Optional Celestia configuration (defaults to CHAINS.celestia)
 * @returns The forwarding address result
 */
export function computeForwardingAddress(
  intent: ForwardingIntent,
  celestiaConfig = CHAINS.celestia
): ForwardingAddressResult {
  // Compute the forwarding salt
  const salt = computeForwardingSalt(intent);
  const callDigest = computeCallDigest(intent);

  // Derive the Celestia module account address
  const addressBytes = deriveCelestiaModuleAddress(celestiaConfig.icaModule, salt);
  const addressBytes32 = "0x" + "00".repeat(12) + bytesToHex(addressBytes).slice(2);

  // Convert to bech32 Celestia address
  const celestiaAddress = toBech32(celestiaConfig.bech32Prefix, addressBytes);

  return {
    celestiaAddress,
    addressBytes32,
    salt,
    callDigest,
  };
}

/**
 * Validates that a Celestia address matches the expected forwarding intent
 * 
 * @param address The address to validate (bech32 or bytes32)
 * @param intent The expected forwarding intent
 * @returns True if the address matches
 */
export function validateForwardingAddress(
  address: string,
  intent: ForwardingIntent
): boolean {
  const expected = computeForwardingAddress(intent);
  
  // Handle both bech32 and bytes32 formats
  if (address.startsWith("celestia")) {
    return address === expected.celestiaAddress;
  } else {
    return address.toLowerCase() === expected.addressBytes32.toLowerCase();
  }
}

/**
 * Converts an EVM address to bytes32 (zero-padded)
 * @param address The EVM address (20 bytes hex string)
 * @returns The address as bytes32 hex string
 */
export function addressToBytes32(address: string): string {
  // Remove 0x prefix if present
  const clean = address.startsWith("0x") ? address.slice(2) : address;
  // Pad to 64 hex chars (32 bytes)
  return "0x" + clean.padStart(64, "0");
}

/**
 * Converts a bytes32 to EVM address (takes last 20 bytes)
 * @param bytes32 The bytes32 hex string
 * @returns The EVM address
 */
export function bytes32ToAddress(bytes32: string): string {
  const clean = bytes32.startsWith("0x") ? bytes32.slice(2) : bytes32;
  return "0x" + clean.slice(-40);
}

