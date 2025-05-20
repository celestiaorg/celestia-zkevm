// This CLI provides basic functionality for deploying Hyperlane components to cosmosnative module.
// Currently the CLI consists of a single command: hyp deploy [grpc-addr]
//
// The deploy command creates a new transaction broadcaster with the provided gRPC address and attempts
// to deploy the following Hyperlane components:
// - NoopISM
// - Mailbox
// - NoopHooks
// - CollateralToken
//
// The CLI can be extended or refactored to adjust logic as necessary.
//
// NOTE: This CLI can be deprecated or removed when the official Hyperlane CLI provides integration support
// with the cosmosnative module.
package cmd
