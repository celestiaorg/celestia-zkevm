/**
 * Constants for ICA-based forwarding
 */

// Version prefix for forwarding address derivation
export const FORWARD_VERSION = "CELESTIA_ICA_FORWARD_V1";

// Empty salt (32 zero bytes)
export const EMPTY_SALT = "0x" + "00".repeat(32);

// Chain configurations
export const CHAINS = {
  rethlocal: {
    name: "rethlocal",
    domainId: 1234,
    rpcUrl: "http://localhost:8545",
    mailbox: "0xb1c938F5BA4B3593377F399e12175e8db0C787Ff",
    icaRouter: "0x4dc4E8bf5D0390C95Af9AFEb1e9c9927c4dB83e7",
    icaIsm: "0x9F098AE0AC3B7F75F0B3126f471E5F592b47F300",
    warpRoute: "0x345a583028762De4d733852c9D4f419077093A48",
  },
  celestia: {
    name: "celestia",
    domainId: 69420,
    rpcUrl: "http://localhost:26657",
    grpcUrl: "http://localhost:9090",
    mailbox: "0x68797065726c616e650000000000000000000000000000000000000000000000",
    ism: "0x726f757465725f69736d00000000000000000000000000000000000000000000",
    // ICA module identifier (to be updated when module is deployed)
    icaModule: "0x6963615f6d6f64756c6500000000000000000000000000000000000000000000",
    icaRouter: "0x6963615f726f7574657200000000000000000000000000000000000000000000",
    // Warp token ID for synthetic TIA
    warpTokenId: "0x726f757465725f61707000000000000000000000000000010000000000000000",
    bech32Prefix: "celestia",
  },
  rethlocal2: {
    name: "rethlocal2",
    domainId: 5678, // TBD - placeholder
    rpcUrl: "http://localhost:9545",
    mailbox: "", // To be deployed
    warpRoute: "", // To be deployed
  },
} as const;

// Default gas limits
export const GAS_LIMITS = {
  ICA_CALL: 500000n,
  WARP_TRANSFER: 200000n,
  FORWARD_EXECUTE: 300000n,
} as const;

