# Celestia gRPC Client

A Rust gRPC client for submitting state transition and state inclusion proofs to the Celestia consensus network. This crate reuses the [Lumina gRPC library](https://github.com/eigerco/lumina/tree/main/grpc) for underlying communication with Celestia validator nodes.

## Features

- Submit state transition proofs for ZK execution ISM
- Submit state inclusion proofs for message submission
- Built on top of the Lumina gRPC library
- Support for both direct Lumina integration and CosmRS transaction building
- CLI tool for proof submission
- Environment-based configuration

## Usage

### Library Usage

```rust
use celestia_grpc_client::{
    CelestiaProofClient, ProofSubmitter, StateTransitionProofMsg, StateInclusionProofMsg,
    ClientConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client from environment variables
    let client = CelestiaProofClient::from_env().await?;

    // Or create with custom config
    let config = ClientConfig {
        grpc_endpoint: "http://localhost:9090".to_string(),
        private_key_hex: "your_private_key_hex".to_string(),
        chain_id: "celestia-zkevm-testnet".to_string(),
        gas_price: 1000,
        max_gas: 200_000,
        confirmation_timeout: 60,
    };
    let client = CelestiaProofClient::new(config).await?;

    // Submit state transition proof
    let state_transition_proof = StateTransitionProofMsg::new(
        "client_id".to_string(),
        proof_bytes,
        public_values,
        target_height,
        prev_state_root,
        new_state_root,
    );

    let response = client.submit_state_transition_proof(state_transition_proof).await?;
    println!("TX Hash: {}", response.tx_hash);

    // Submit state inclusion proof
    let state_inclusion_proof = StateInclusionProofMsg::new(
        "client_id".to_string(),
        key_paths,
        proof_bytes,
        height,
        state_root,
        values,
    );

    let response = client.submit_state_inclusion_proof(state_inclusion_proof).await?;
    println!("TX Hash: {}", response.tx_hash);

    Ok(())
}
```

### CLI Usage

The crate includes a CLI tool for submitting proofs:

```bash
# Set environment variables
export CELESTIA_GRPC_ENDPOINT="http://localhost:9090"
export CELESTIA_PRIVATE_KEY="your_private_key_hex"
export CELESTIA_CHAIN_ID="celestia-zkevm-testnet"

# Submit state transition proof
cargo run --bin proof_submitter -- state-transition \
    --client-id "0x123..." \
    --proof-file "./proof.hex" \
    --public-values-file "./public_values.hex" \
    --target-height 1000 \
    --prev-state-root "0xabc..." \
    --new-state-root "0xdef..."

# Submit state inclusion proof
cargo run --bin proof_submitter -- state-inclusion \
    --client-id "0x123..." \
    --key-paths "key1,key2,key3" \
    --proof-file "./proof.hex" \
    --height 1000 \
    --state-root "0xabc..." \
    --values-file "./values.hex"

# Check account balance
cargo run --bin proof_submitter -- balance
```

## Environment Variables

- `CELESTIA_GRPC_ENDPOINT`: Celestia validator gRPC endpoint (default: `http://localhost:9090`)
- `CELESTIA_PRIVATE_KEY`: Private key for signing transactions (hex encoded, required)
- `CELESTIA_CHAIN_ID`: Chain ID for the Celestia network (default: `celestia-zkevm-testnet`)
- `CELESTIA_GAS_PRICE`: Gas price for transactions (default: `1000`)
- `CELESTIA_MAX_GAS`: Maximum gas limit per transaction (default: `200000`)
- `CELESTIA_CONFIRMATION_TIMEOUT`: Timeout for transaction confirmation in seconds (default: `60`)

## Features

- `cosmrs-support` (default): Enable CosmRS transaction building support
- Without this feature, the client uses direct Lumina message submission

## Dependencies

This crate integrates with:
- [Lumina gRPC](https://github.com/eigerco/lumina/tree/main/grpc) - For Celestia validator communication
- [CosmRS](https://github.com/cosmos/cosmos-rust) - For Cosmos SDK transaction building (optional)
- Celestia types from the workspace

## Implementation Status

**✅ Currently Working:**
- Message structure validation based on actual Celestia PR definitions
- Lumina gRPC client integration
- CLI interface with correct message fields
- Type-safe proof message handling
- Comprehensive error handling and validation

**🚧 Placeholder Implementation:**
- Actual proof submission (returns placeholder transaction hashes)
- The implementation is ready to integrate with Celestia once the zkISM module is deployed

**🔮 Future Integration:**
- Direct zkISM transaction submission via Celestia's `/broadcast_tx_sync` endpoint
- Real transaction hash extraction from Celestia responses
- Gas estimation integration with Celestia fee markets

## Transaction Types

The client supports the exact message types from Celestia PRs:

1. **`MsgUpdateZKExecutionISM`** - State transition proof updates
   - From [celestia-app#5788](https://github.com/celestiaorg/celestia-app/pull/5788)
   - Fields: `id`, `height`, `proof`, `public_values`

2. **`MsgSubmitMessages`** - State membership proof submission
   - From [celestia-app#5790](https://github.com/celestiaorg/celestia-app/pull/5790)
   - Fields: `id`, `height`, `proof`, `public_values`
