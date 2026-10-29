/**
 * Types for ICA-based forwarding
 */

/**
 * A forwarding intent specifies where tokens should ultimately be delivered
 */
export interface ForwardingIntent {
  /** The warp token ID on Celestia (bytes32 hex string) */
  tokenId: string;
  /** The final destination domain ID */
  destDomain: number;
  /** The final recipient address (bytes32 hex string, zero-padded for EVM addresses) */
  destRecipient: string;
  /** Optional: amount to forward (for validation/display purposes) */
  amount?: bigint;
}

/**
 * Parameters for deriving an ICA address
 */
export interface ICAAddressParams {
  /** The origin domain ID */
  origin: number;
  /** The owner address (bytes32 hex string) */
  owner: string;
  /** The router address (bytes32 hex string) */
  router: string;
  /** The ISM address (bytes32 hex string) */
  ism: string;
  /** Optional user-provided salt (bytes32 hex string) */
  userSalt?: string;
}

/**
 * A call to be executed via ICA
 */
export interface ICACall {
  /** Target address (bytes32 hex string) */
  to: string;
  /** Value to send (in wei) */
  value: bigint;
  /** Call data (hex string) */
  data: string;
}

/**
 * ICA message types
 */
export enum ICAMessageType {
  CALLS = 0,
  COMMITMENT = 1,
  REVEAL = 2,
}

/**
 * An encoded ICA message
 */
export interface ICAMessage {
  /** Message type */
  type: ICAMessageType;
  /** Message body (hex string) */
  body: string;
}

/**
 * Chain configuration
 */
export interface ChainConfig {
  /** Chain name */
  name: string;
  /** Domain ID */
  domainId: number;
  /** RPC URL */
  rpcUrl: string;
  /** Mailbox address */
  mailbox: string;
  /** ICA Router address (if deployed) */
  icaRouter?: string;
  /** Warp route address (if deployed) */
  warpRoute?: string;
}

/**
 * Celestia-specific configuration
 */
export interface CelestiaConfig extends ChainConfig {
  /** gRPC URL */
  grpcUrl: string;
  /** ICA module address (bytes32) */
  icaModule: string;
  /** Warp module token ID (bytes32) */
  warpTokenId: string;
}

/**
 * Forwarding address computation result
 */
export interface ForwardingAddressResult {
  /** The computed forwarding address (Celestia bech32 format) */
  celestiaAddress: string;
  /** The forwarding address as bytes32 */
  addressBytes32: string;
  /** The salt used for derivation */
  salt: string;
  /** The call digest */
  callDigest: string;
}

