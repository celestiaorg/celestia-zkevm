# Celestia gRPC Client

This crate contains a basic gRPC client used for transaction submission and querying of the celestia zkism module.

## Protobuf

Protobuf is used by Celestia as the canonical encoding format and thus we leverage this for RPC messaging.
In order to interact with the `x/zkism` module we include the Protobuf definition in this crate under the `proto` directory.

The `buf` toolchain is employed to handle Rust code generation. 
Please refer to the [official installation documentation](https://buf.build/docs/cli/installation/) to get setup with the `buf` CLI.

Rust code-gen is produced from the Protobuf defintions via `buf.gen.yaml` plugins and included in this crate under `src/proto`.

### Usage

1. Generate the `celestia-grpc-client` code by running the following command:

```bash
cd proto
buf generate --template buf.gen.yaml
```

2. Generate the CosmosSDK dependencies by running the following command:

```bash
cd proto
buf generate --template buf.gen.yaml \
  buf.build/cosmos/cosmos-sdk:aa25660f4ff746388669ce36b3778442 \
  --path cosmos/base/v1beta1/coin.proto \
  --path cosmos/base/query/v1beta1/pagination.proto \
  buf.build/bcp-innovations/hyperlane-cosmos:v1.0.1 \
  --path hyperlane/core/v1/tx.proto
```

3. Generate the Hyperlane dependencies by running the following command:

```bash
buf generate --template buf.gen.yaml \
  buf.build/bcp-innovations/hyperlane-cosmos:v1.0.1 \
  --path hyperlane/core/v1/tx.proto
```

3. Update module dependencies:

```bash
buf dep update
```
