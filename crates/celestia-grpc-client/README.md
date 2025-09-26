# Celestia gRPC Client

A command-line tool for submitting zero-knowledge proofs to Celestia via gRPC. This tool enables the submission of state transition proofs and state inclusion proofs for the ZK Interchain Security Module (ISM).

## Overview

The `proof_submitter` binary provides a CLI interface for:

- **State Transition Proofs**: Submit proofs that verify EVM state transitions (MsgUpdateZKExecutionISM)
- **State Inclusion Proofs**: Submit proofs that verify message inclusion in state (MsgSubmitMessages)

## Installation

### Build from Source

```bash
cargo build --release --bin proof_submitter
```

The binary will be available at `target/release/proof_submitter`.

## Configuration

The tool uses environment variables for configuration. Set the following variables:

### Required Environment Variables

```bash
# Celestia private key (hex encoded, 64 characters)
export CELESTIA_PRIVATE_KEY="your_private_key_here"

# Celestia gRPC endpoint
export CELESTIA_GRPC_ENDPOINT="http://localhost:9090"
```

### Optional Environment Variables

```bash
# Chain ID (default: celestia-zkevm-testnet)
export CELESTIA_CHAIN_ID="celestia-zkevm-testnet"

# Gas price (default: 1000)
export CELESTIA_GAS_PRICE="1000"

# Maximum gas limit (default: 200000)
export CELESTIA_MAX_GAS="200000"

# Confirmation timeout in seconds (default: 60)
export CELESTIA_CONFIRMATION_TIMEOUT="60"
```

## Usage

### State Transition Proof Submission

Submit a proof that verifies an EVM state transition:

```bash
proof_submitter state-transition \
  --id "ism-001" \
  --proof-file "proof.hex" \
  --public-values-file "public_values.hex" \
  --height 12345
```

**Parameters:**
- `--id`: ISM identifier (string)
- `--proof-file`: Path to hex-encoded proof file
- `--public-values-file`: Path to hex-encoded public values file
- `--height`: Block height for the state transition (u64)

### State Inclusion Proof Submission

Submit a proof that verifies message inclusion in state:

```bash
proof_submitter state-inclusion \
  --id "ism-001" \
  --proof-file "inclusion_proof.hex" \
  --public-values-file "inclusion_public_values.hex" \
  --height 12345
```

**Parameters:**
- `--id`: ISM identifier (string)
- `--proof-file`: Path to hex-encoded proof file
- `--public-values-file`: Path to hex-encoded public values file
- `--height`: Block height for the inclusion proof (u64)

## File Format

### Proof Files

Proof files must contain hex-encoded binary data:

```
# Example proof.hex
0x1234567890abcdef...
```

### Public Values Files

Public values files must contain hex-encoded binary data:

```
# Example public_values.hex
0xabcdef1234567890...
```

## Output

On successful submission, the tool displays:

```
State transition proof submitted successfully!
Transaction hash: ABC123...
Block height: 12345
Gas used: 150000
```

## Error Handling

The tool provides detailed error messages for common issues:

- **Missing environment variables**: Clear indication of which variables are required
- **Invalid file paths**: File not found or unreadable
- **Invalid hex encoding**: Malformed hex data in proof files
- **gRPC connection errors**: Network connectivity issues
- **Transaction failures**: Celestia-specific error messages

## Examples

### Complete Workflow

1. **Set environment variables:**
```bash
export CELESTIA_PRIVATE_KEY="0123456789abcdef..."
export CELESTIA_GRPC_ENDPOINT="http://localhost:9090"
```

2. **Prepare proof files:**
```bash
# Generate your proof and public values
# Save as hex-encoded files
echo "0x1234..." > proof.hex
echo "0xabcd..." > public_values.hex
```

3. **Submit state transition proof:**
```bash
proof_submitter state-transition \
  --id "evm-ism-001" \
  --proof-file "proof.hex" \
  --public-values-file "public_values.hex" \
  --height 1000
```

4. **Submit state inclusion proof:**
```bash
proof_submitter state-inclusion \
  --id "evm-ism-001" \
  --proof-file "inclusion_proof.hex" \
  --public-values-file "inclusion_public_values.hex" \
  --height 1000
```

## Integration with EV Prover

This tool is designed to work with the EV Prover service:

1. **Generate proofs** using the EV Prover service
2. **Extract proof data** from the prover's output
3. **Submit proofs** using this CLI tool
4. **Verify submission** through the transaction hash

## Troubleshooting

### Common Issues

1. **"CELESTIA_PRIVATE_KEY environment variable not set"**
   - Ensure the private key is set and properly formatted (64 hex characters)

2. **"Invalid hex encoding"**
   - Verify proof files contain valid hex data
   - Remove any whitespace or newlines from hex files

3. **"Failed to connect to gRPC endpoint"**
   - Check that Celestia node is running and accessible
   - Verify the gRPC endpoint URL is correct

4. **"Transaction failed"**
   - Check account balance for gas fees
   - Verify the proof data is valid
   - Check Celestia node logs for detailed error information

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug proof_submitter state-transition --id "test" --proof-file "proof.hex" --public-values-file "values.hex" --height 100
```

## Security Considerations

- **Private Key Security**: Never commit private keys to version control
- **Network Security**: Use secure connections (HTTPS/TLS) in production
- **Proof Validation**: Always verify proofs before submission
- **Gas Limits**: Set appropriate gas limits to prevent failed transactions

## API Reference

### Message Types

#### StateTransitionProofMsg
- `id`: ISM identifier
- `height`: Block height
- `proof`: ZK proof bytes
- `public_values`: Public inputs/outputs

#### StateInclusionProofMsg
- `id`: ISM identifier  
- `height`: Block height
- `proof`: ZK proof bytes
- `public_values`: Public inputs/outputs

### Response Format

```json
{
  "tx_hash": "string",
  "height": 12345,
  "gas_used": 150000,
  "success": true,
  "error_message": null
}
```

## Contributing

See the main repository [CONTRIBUTING.md](../../docs/CONTRIBUTING.md) for guidelines.

## License

This project is part of the Celestia ecosystem. See the main repository for license information.