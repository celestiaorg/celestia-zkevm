// This CLI provides functionality for deploying and managing Hyperlane components on cosmosnative modules.
//
// Available commands:
//   - deploy-noopism [grpc-addr] [local-domain]: Deploy with NoopISM (for testing)
//   - deploy-zkism [grpc-addr] [evm-rpc] [ev-node-rpc] [local-domain]: Deploy with ZK Execution ISM
//   - deploy-multisigism [grpc-addr] [validators] [threshold] [local-domain]: Deploy with Multisig ISM
//   - enroll-remote-router [grpc-addr] [token-id] [remote-domain] [remote-contract]: Configure remote router
//   - setup-zkism [grpc-addr] [evm-rpc] [ev-node-rpc]: Deploy and configure new ZK ISM with existing stack
//   - announce-validator [grpc-addr] [validator] [storage-location] [signature] [mailbox-id]: Announce validator
//
// Each deployment command creates a transaction broadcaster with the provided gRPC address and deploys
// the following Hyperlane components:
// - ISM (NoopISM, ZK Execution ISM, or Multisig ISM)
// - Mailbox
// - NoopHooks
// - CollateralToken
//
// NOTE: This CLI can be deprecated or removed when the official Hyperlane CLI provides integration support
// with the cosmosnative module.
package cmd
