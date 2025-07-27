## TxFlood CLI tool

A simple CLI tool to create, fund, and send transactions on EVM nodes using multiple generated accounts.
This tool is intended to be used in development environments for testing purposes.

### Installation

Navigate to this directory and run:

```shell
go install ./cmd/txflood
```

### Usage

This tool requires a well-funded faucet account.

1. Create test user accounts for sending transactions.

```shell
txflood create-accounts [num-accounts]
```

2. Fund the accounts, providing the private key of the faucet account.

```shell
txflood fund-accounts [faucet-key]
```

3. Send `num-txs` transactions between accounts using a round-robin format.

```shell
txflood send-txs [num-txs]
```

4. Start a transactions send-loop with a configurable interval and fixed number of transactions.

```shell
txflood flood --interval 5s --num-txs 10
```

5. Start a transactions send-loop with a configurable interval and randomised number of transactions with `num-txs` as the upper bound.

```shell
txflood flood --interval 5s --num-txs 50 --randomise
```
