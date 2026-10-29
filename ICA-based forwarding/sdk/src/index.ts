/**
 * ICA-based Forwarding SDK
 * 
 * This SDK provides utilities for computing Celestia ICA forwarding addresses
 * and building transactions for the ICA-based token forwarding pattern.
 */

// Types
export * from "./types";

// Constants
export * from "./constants";

// Address computation
export {
  computeForwardingSalt,
  computeCallDigest,
  deriveCelestiaModuleAddress,
  computeICAAddress,
  computeForwardingAddress,
  validateForwardingAddress,
  addressToBytes32,
  bytes32ToAddress,
} from "./address";

// Encoding
export {
  encodeForwardingCallData,
  buildForwardingCall,
  encodeICACalls,
  encodeICACallsMessage,
  encodeICACommitmentMessage,
  computeCommitment,
  decodeICAMessage,
  encodeWarpPayload,
} from "./encoding";

// Re-export for convenience
import { ForwardingIntent, ForwardingAddressResult } from "./types";
import { computeForwardingAddress, addressToBytes32 } from "./address";
import { buildForwardingCall, encodeICACallsMessage } from "./encoding";
import { CHAINS, EMPTY_SALT } from "./constants";

/**
 * High-level helper to compute a forwarding address and build the ICA call
 * 
 * @param tokenId The warp token ID on Celestia
 * @param destDomain The final destination domain
 * @param destRecipient The final recipient (EVM address or bytes32)
 * @param amount The amount to forward
 * @returns Object containing forwarding address and ICA message
 */
export function prepareForwarding(
  tokenId: string,
  destDomain: number,
  destRecipient: string,
  amount: bigint
): {
  forwardingAddress: ForwardingAddressResult;
  icaCall: ReturnType<typeof buildForwardingCall>;
  icaMessage: ReturnType<typeof encodeICACallsMessage>;
} {
  // Normalize recipient to bytes32
  const recipientBytes32 = destRecipient.length === 42 
    ? addressToBytes32(destRecipient)
    : destRecipient;

  const intent: ForwardingIntent = {
    tokenId,
    destDomain,
    destRecipient: recipientBytes32,
    amount,
  };

  // Compute forwarding address
  const forwardingAddress = computeForwardingAddress(intent);

  // Build the ICA call
  const icaCall = buildForwardingCall(
    CHAINS.celestia.warpTokenId, // warp module address
    intent,
    amount
  );

  // Build the ICA message (using the forwarding address as the derived ICA)
  const icaMessage = encodeICACallsMessage(
    forwardingAddress.addressBytes32, // owner
    CHAINS.celestia.ism, // ISM
    [icaCall],
    forwardingAddress.salt // use forwarding salt as user salt
  );

  return {
    forwardingAddress,
    icaCall,
    icaMessage,
  };
}

