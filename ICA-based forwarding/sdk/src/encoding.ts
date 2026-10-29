/**
 * Encoding functions for ICA messages and calls
 */

import { 
  encodeAbiParameters, 
  parseAbiParameters, 
  concat, 
  toHex,
  hexToBytes,
  keccak256,
} from "viem";
import { ForwardingIntent, ICACall, ICAMessage, ICAMessageType } from "./types";
import { EMPTY_SALT } from "./constants";
import { addressToBytes32 } from "./address";

/**
 * Encodes a forwarding call for the Celestia warp module
 * 
 * @param intent The forwarding intent
 * @param amount The amount of tokens to forward
 * @returns The encoded call data
 */
export function encodeForwardingCallData(
  intent: ForwardingIntent,
  amount: bigint
): string {
  // Encode the RemoteTransfer call for hyperlane-cosmos warp module
  // This matches the MsgRemoteTransfer message format
  return encodeAbiParameters(
    parseAbiParameters("bytes32, uint32, bytes32, uint256"),
    [
      intent.tokenId as `0x${string}`,
      intent.destDomain,
      intent.destRecipient as `0x${string}`,
      amount,
    ]
  );
}

/**
 * Builds an ICA Call struct for forwarding
 * 
 * @param warpModuleAddress The Celestia warp module address (bytes32)
 * @param intent The forwarding intent
 * @param amount The amount to forward
 * @returns The ICA Call
 */
export function buildForwardingCall(
  warpModuleAddress: string,
  intent: ForwardingIntent,
  amount: bigint
): ICACall {
  return {
    to: warpModuleAddress,
    value: 0n,
    data: encodeForwardingCallData(intent, amount),
  };
}

/**
 * Encodes an array of ICA Calls in the format expected by InterchainAccountRouter
 * 
 * @param calls The calls to encode
 * @returns The ABI-encoded calls
 */
export function encodeICACalls(calls: ICACall[]): string {
  // CallLib.Call[] is encoded as a dynamic array of structs
  const callTuples = calls.map((call) => ({
    to: call.to as `0x${string}`,
    value: call.value,
    data: call.data as `0x${string}`,
  }));

  return encodeAbiParameters(
    parseAbiParameters("(bytes32 to, uint256 value, bytes data)[]"),
    [callTuples]
  );
}

/**
 * Encodes an ICA CALLS message
 * 
 * Format:
 * [0:1]   MessageType.CALLS (uint8)
 * [1:33]  ICA owner (bytes32)
 * [33:65] ICA ISM (bytes32)
 * [65:97] User Salt (bytes32)
 * [97:?]  Calls (CallLib.Call[]), abi encoded
 * 
 * @param owner The owner address (bytes32)
 * @param ism The ISM address (bytes32)
 * @param calls The calls to execute
 * @param userSalt Optional user salt
 * @returns The encoded message
 */
export function encodeICACallsMessage(
  owner: string,
  ism: string,
  calls: ICACall[],
  userSalt: string = EMPTY_SALT
): ICAMessage {
  const messageTypeByte = toHex(new Uint8Array([ICAMessageType.CALLS]));
  const encodedCalls = encodeICACalls(calls);

  const body = concat([
    messageTypeByte as `0x${string}`,
    owner as `0x${string}`,
    ism as `0x${string}`,
    userSalt as `0x${string}`,
    encodedCalls as `0x${string}`,
  ]);

  return {
    type: ICAMessageType.CALLS,
    body,
  };
}

/**
 * Encodes an ICA COMMITMENT message
 * 
 * Format:
 * [0:1]   MessageType.COMMITMENT (uint8)
 * [1:33]  ICA owner (bytes32)
 * [33:65] ICA ISM (bytes32)
 * [65:97] User Salt (bytes32)
 * [97:129] Commitment (bytes32)
 * 
 * @param owner The owner address (bytes32)
 * @param ism The ISM address (bytes32)
 * @param commitment The commitment hash
 * @param userSalt Optional user salt
 * @returns The encoded message
 */
export function encodeICACommitmentMessage(
  owner: string,
  ism: string,
  commitment: string,
  userSalt: string = EMPTY_SALT
): ICAMessage {
  const messageTypeByte = toHex(new Uint8Array([ICAMessageType.COMMITMENT]));

  const body = concat([
    messageTypeByte as `0x${string}`,
    owner as `0x${string}`,
    ism as `0x${string}`,
    userSalt as `0x${string}`,
    commitment as `0x${string}`,
  ]);

  return {
    type: ICAMessageType.COMMITMENT,
    body,
  };
}

/**
 * Computes a commitment hash for a set of calls
 * 
 * commitment = keccak256(abi.encodePacked(salt, abi.encode(calls)))
 * 
 * @param calls The calls to commit
 * @param salt The salt for the commitment
 * @returns The commitment hash
 */
export function computeCommitment(calls: ICACall[], salt: string): string {
  const encodedCalls = encodeICACalls(calls);
  return keccak256(
    concat([salt as `0x${string}`, encodedCalls as `0x${string}`])
  );
}

/**
 * Decodes an ICA message body
 * 
 * @param body The message body (hex string)
 * @returns Parsed message components
 */
export function decodeICAMessage(body: string): {
  type: ICAMessageType;
  owner?: string;
  ism?: string;
  salt?: string;
  calls?: ICACall[];
  commitment?: string;
} {
  const bytes = hexToBytes(body as `0x${string}`);
  const type = bytes[0] as ICAMessageType;

  if (type === ICAMessageType.CALLS) {
    const owner = "0x" + Buffer.from(bytes.slice(1, 33)).toString("hex");
    const ism = "0x" + Buffer.from(bytes.slice(33, 65)).toString("hex");
    const salt = "0x" + Buffer.from(bytes.slice(65, 97)).toString("hex");
    // Calls are ABI encoded starting at byte 97
    // For simplicity, we don't decode the calls here
    return { type, owner, ism, salt };
  } else if (type === ICAMessageType.COMMITMENT) {
    const owner = "0x" + Buffer.from(bytes.slice(1, 33)).toString("hex");
    const ism = "0x" + Buffer.from(bytes.slice(33, 65)).toString("hex");
    const salt = "0x" + Buffer.from(bytes.slice(65, 97)).toString("hex");
    const commitment = "0x" + Buffer.from(bytes.slice(97, 129)).toString("hex");
    return { type, owner, ism, salt, commitment };
  } else if (type === ICAMessageType.REVEAL) {
    const ism = "0x" + Buffer.from(bytes.slice(1, 33)).toString("hex");
    const commitment = "0x" + Buffer.from(bytes.slice(33, 65)).toString("hex");
    return { type, ism, commitment };
  }

  return { type };
}

/**
 * Encodes a Hyperlane warp transfer message body
 * 
 * Standard warp payload format:
 * [0:32]  recipient (bytes32)
 * [32:64] amount (uint256)
 * 
 * @param recipient The recipient address (bytes32)
 * @param amount The amount to transfer
 * @returns The encoded payload
 */
export function encodeWarpPayload(recipient: string, amount: bigint): string {
  return encodeAbiParameters(
    parseAbiParameters("bytes32, uint256"),
    [recipient as `0x${string}`, amount]
  );
}

