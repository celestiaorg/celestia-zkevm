# Counterfactual ICA-Based Forwarding (Call-Bound, CEX-Compatible)

## Overview
Assets arriving on Celestia (from Hyperlane chains or CEX withdrawals) are forwarded to a final destination using a counterfactual interchain account (ICA) bound to a specific forwarding intent. The forwarding address is derived from a cryptographic commitment to the destination domain, recipient, and token. Funds can only be forwarded according to this commitment.

## Address Derivation
The forwarding intent is encoded as:

```go
forwardCall = transferRemote(token, destDomain, destRecipient)
callDigest = H(encode(forwardCall))
salt = H("CELESTIA_ICA_FORWARD_V1", callDigest)
// owner = Celestia forwarding router identity
forwardAddr = ICAAddress(owner, salt)
```

This binds the address to the destination, recipient, and token. The `owner` defines who may authorize ICA execution; it does not hold funds or sign transactions.

## Flow
1) Off-chain system computes `forwardAddr`.
2) User transfers assets to `forwardAddr` via Hyperlane warp transfer or CEX withdrawal.
3) Assets are credited to `forwardAddr`.
4) A relayer (or any permissionless actor) submits an ICA execution request through Hyperlane.
5) ICA verifies the call matches `callDigest` and dispatches the entire balance held at `forwardAddr` to the final destination.

## Security
- Funds can only be forwarded to the committed destination and recipient.
- ICA execution messages are processed at most once by the Hyperlane Mailbox.
- Malicious executors cannot redirect funds or execute arbitrary calls.

## Failure & Recovery
- If no ICA execution is submitted, funds remain safely held at `forwardAddr`.
- If an invalid execution is attempted, it fails deterministically with no state change.
- Execution may be retried permissionlessly until successful.

At no point can funds be forwarded to an unintended destination.