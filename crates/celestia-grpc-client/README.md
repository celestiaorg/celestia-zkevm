# Celestia gRPC Client

This crate contains a basic gRPC client used for transaction submission and querying of the celestia zkism module.

## Protobuf

Protobuf is used by Celestia as the canonical encoding format and thus we leverage this for RPC messaging.
In order to interact with the `x/zkism` module we include the Protobuf definition in this crate under the `proto` directory.

The `buf` toolchain is employed to handle Rust code generation. Please refer to the [official installation documentation](https://buf.build/docs/cli/installation/) to get setup with the `buf` CLI.

### Usage

1. Regenerate the Rust celestia-grpc-client code by running the following command:

```bash
buf generate --template buf.gen.yaml proto
```

2. Regenerate the Rust cosmos dependencies by running:

```bash
cd proto
buf generate \
  --template buf.gen.yaml \
  buf.build/cosmos/cosmos-sdk:aa25660f4ff746388669ce36b3778442 \
  --path cosmos/base/v1beta1/coin.proto \
  --path cosmos/base/query/v1beta1/pagination.proto
```

3. Update module dependencies

```bash
buf dep update
```
