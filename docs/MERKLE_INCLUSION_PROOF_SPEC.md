# Merkle Inclusion Proof Specification

## Overview

This document specifies a dual merkle inclusion proof system for verifying cross-chain messages from EVM chains to Celestia. This system works in conjunction with an existing ZK proof system that verifies EVM state roots, providing complete cryptographic verification of cross-chain messages without requiring ZK proofs for the message inclusion itself.

## Motivation

The current NoopISM implementation provides no security guarantees, while generating ZK proofs for every message inclusion would be computationally expensive and unnecessary. This dual merkle inclusion proof system offers:

- **Cryptographic verification** that messages were actually dispatched on the source chain
- **Efficient verification** using standard merkle proof algorithms 
- **Optimal security/performance balance** by leveraging existing ZK state root verification
- **Complementary design** that works with the existing ZK proof system for EVM state roots

## Architecture

### Proof Chain Overview

```
Message ID → Mailbox Root → EVM State Root → ZK Verified State
     ↑            ↑              ↑              ↑
   Proof 1      Proof 2      Proof 3      ZK System
 (Merkle)     (Merkle)    (ZK Proof)   (Existing)
```

The complete verification process involves:

1. **Mailbox Inclusion Proof**: Proves a specific message ID exists in the Hyperlane mailbox's merkle tree
2. **MPT Storage Inclusion Proof**: Proves the mailbox root exists in the contract's storage using Merkle Patricia Tree proofs  
3. **State Root Verification**: The existing ZK proof system verifies the EVM state root is correct

### System Integration

This merkle inclusion system is designed to work seamlessly with the existing ZK proof infrastructure:

- **ZK System**: Handles computationally intensive state root verification
- **Merkle System**: Handles efficient message inclusion verification within verified state
- **Combined Security**: Full cryptographic verification without redundant ZK proving

## Data Structures

### Core Types

```go
import (
    "github.com/ethereum/go-ethereum/common"
    "github.com/ethereum/go-ethereum/common/hexutil"
    "github.com/ethereum/go-ethereum/ethclient"
    "github.com/ethereum/go-ethereum/trie"
    "github.com/ethereum/go-ethereum/core/rawdb"
    "github.com/ethereum/go-ethereum/ethdb"
    "github.com/ethereum/go-ethereum/crypto"
    "github.com/ethereum/go-ethereum/rlp"
)

// MessageID represents a unique identifier for a dispatched message
type MessageID [32]byte

// MerkleRoot represents a 32-byte merkle tree root
type MerkleRoot [32]byte

// MerkleProof represents a simple merkle inclusion proof (for mailbox tree)
type MerkleProof struct {
    // Leaf is the value being proven (message ID)
    Leaf [32]byte `json:"leaf"`
    
    // Index is the position of the leaf in the tree
    Index uint64 `json:"index"`
    
    // Siblings are the sibling hashes needed for verification
    Siblings [][32]byte `json:"siblings"`
    
    // TreeSize is the total number of leaves in the tree
    TreeSize uint64 `json:"tree_size"`
}

// MPTProof represents a Merkle Patricia Tree proof for EVM storage
type MPTProof struct {
    // AccountProof proves the contract account exists in the state trie
    AccountProof []hexutil.Bytes `json:"accountProof"`
    
    // Address is the contract address being proven
    Address common.Address `json:"address"`
    
    // Balance of the contract account
    Balance *hexutil.Big `json:"balance"`
    
    // CodeHash of the contract
    CodeHash common.Hash `json:"codeHash"`
    
    // Nonce of the contract account
    Nonce hexutil.Uint64 `json:"nonce"`
    
    // StorageHash is the root of the contract's storage trie
    StorageHash common.Hash `json:"storageHash"`
    
    // StorageProof proves the specific storage slot exists
    StorageProof []hexutil.Bytes `json:"storageProof"`
    
    // StorageKey is the key being proven in storage
    StorageKey common.Hash `json:"storageKey"`
    
    // StorageValue is the value at the storage key
    StorageValue hexutil.Big `json:"storageValue"`
}

// Supporting types for eth_getProof response
type EthGetProofResponse struct {
    AccountProof []hexutil.Bytes `json:"accountProof"`
    Address      common.Address  `json:"address"`
    Balance      *hexutil.Big    `json:"balance"`
    CodeHash     common.Hash     `json:"codeHash"`
    Nonce        hexutil.Uint64  `json:"nonce"`
    StorageHash  common.Hash     `json:"storageHash"`
    StorageProof []StorageProof  `json:"storageProof"`
}

type StorageProof struct {
    Key   common.Hash       `json:"key"`
    Value hexutil.Big       `json:"value"`
    Proof []hexutil.Bytes   `json:"proof"`
}
```

### Combined Metadata Structure

```go
// DualMerkleInclusionMetadata contains both inclusion proofs
type DualMerkleInclusionMetadata struct {
    // MessageToMailboxProof proves message ID is in mailbox tree
    MessageToMailboxProof MerkleProof `json:"message_to_mailbox_proof"`
    
    // MailboxStorageProof proves mailbox root exists in contract storage
    MailboxStorageProof MPTProof `json:"mailbox_storage_proof"`
    
    // MailboxRoot is the intermediate root connecting both proofs
    MailboxRoot MerkleRoot `json:"mailbox_root"`
    
    // MailboxContractAddress is the address of the Hyperlane mailbox contract
    MailboxContractAddress common.Address `json:"mailbox_contract_address"`
    
    // MailboxStorageSlot is the storage slot containing the tree root
    MailboxStorageSlot common.Hash `json:"mailbox_storage_slot"`
    
    // StateRoot is the EVM state root (verified by ZK system)
    StateRoot MerkleRoot `json:"state_root"`
    
    // BlockNumber for the EVM block containing this state
    BlockNumber uint64 `json:"block_number"`
    
    // BlockHash for additional verification context
    BlockHash [32]byte `json:"block_hash"`
    
    // ZKStateProof contains the ZK proof verifying this state root
    // (Generated by the existing ZK proof system)
    ZKStateProof []byte `json:"zk_state_proof,omitempty"`
}
```

## Proof Generation Process

### Overview: Two-Layer Proof Strategy

This system uses a two-layer proof approach similar to the IBC packet commitment proofs in celestia-zkevm-ibc-demo:

1. **Layer 1 (Mailbox Tree)**: Standard merkle proof that a message ID exists in the Hyperlane mailbox's internal merkle tree
2. **Layer 2 (EVM Storage)**: MPT proof that the mailbox tree root exists in the EVM contract's storage, which rolls up to the state root

### Step 1: Message Dispatch Detection

The relayer monitors the EVM chain for `Dispatch` events from the Hyperlane mailbox contract:

```solidity
event Dispatch(
    address indexed sender,
    uint32 indexed destination,
    bytes32 indexed recipient,
    bytes32 indexed messageId,
    bytes message
);
```

This is analogous to how the IBC demo monitors for packet commitment events.

### Step 2: Mailbox Inclusion Proof Generation

```go
func GenerateMailboxInclusionProof(
    messageID MessageID,
    mailboxContract EthereumContract,
    blockNumber uint64,
) (MerkleProof, MerkleRoot, error) {
    // 1. Query mailbox contract for merkle tree state at block
    tree, err := mailboxContract.GetMerkleTree(blockNumber)
    if err != nil {
        return MerkleProof{}, MerkleRoot{}, err
    }
    
    // 2. Find message ID in tree and generate proof
    proof, err := tree.GenerateProof(messageID)
    if err != nil {
        return MerkleProof{}, MerkleRoot{}, err
    }
    
    // 3. Return proof and tree root
    return proof, tree.Root(), nil
}
```

### Step 3: MPT Storage Proof Generation

```go
// getMailboxMPTProof queries the EVM node for a Merkle Patricia Trie proof 
// proving the mailbox tree root exists in the mailbox contract storage
// Based on celestia-zkevm-ibc-demo/testing/demo/pkg/transfer/eth_utils.go
func getMailboxMPTProof(
    expectedMailboxRoot MerkleRoot,
    mailboxContractAddress string, 
    blockNumber uint64,
) (MPTProof, error) {
    // 1. Calculate the storage slot where mailbox stores its tree root
    mailboxRootStorageSlot := GetMailboxRootStorageSlot()
    
    // 2. Connect to EVM client
    client, err := ethclient.Dial(ethereumRPC)
    if err != nil {
        return MPTProof{}, fmt.Errorf("failed to connect to EVM node: %w", err)
    }
    defer client.Close()
    
    // 3. Generate the MPT proof using eth_getProof
    var result EthGetProofResponse
    err = client.Client().Call(
        &result, 
        "eth_getProof", 
        mailboxContractAddress, 
        []string{mailboxRootStorageSlot.Hex()}, 
        hexutil.EncodeUint64(blockNumber),
    )
    if err != nil {
        return MPTProof{}, fmt.Errorf("failed to get MPT proof: %w", err)
    }
    
    // 4. Find the proof for our specific storage key
    var targetProof StorageProof
    for _, proof := range result.StorageProof {
        if proof.Key == mailboxRootStorageSlot {
            targetProof = proof
            break
        } else {
            return MPTProof{}, fmt.Errorf("proof key does not match expected slot: %x", proof.Key)
        }
    }
    
    // 5. Verify the storage value matches expected mailbox root
    expectedValue := new(big.Int).SetBytes(expectedMailboxRoot[:])
    if targetProof.Value.ToInt().Cmp(expectedValue) != 0 {
        return MPTProof{}, fmt.Errorf("storage value mismatch: expected %x, got %x", 
            expectedValue, targetProof.Value.ToInt())
    }
    
    // 6. Construct the MPT proof (following exact pattern from getMPTProof)
    proof := MPTProof{
        AccountProof: result.AccountProof,
        Address:      result.Address,
        Balance:      result.Balance,
        CodeHash:     result.CodeHash,
        Nonce:        result.Nonce,
        StorageHash:  result.StorageHash,
        StorageProof: targetProof.Proof,
        StorageKey:   targetProof.Key,
        StorageValue: targetProof.Value,
    }
    
    return proof, nil
}

// GetMailboxRootStorageSlot calculates the storage slot for the Hyperlane mailbox tree root
// This follows the same pattern as GetCommitmentsStorageKey for IBC commitments
func GetMailboxRootStorageSlot() common.Hash {
    // For Hyperlane mailbox contracts, the tree root is typically stored in a specific slot
    // This would need to be determined based on the actual Hyperlane mailbox contract
    
    // Example for simple storage (tree root in slot 0):
    return common.HexToHash("0x0")
    
    // Example for more complex storage layout like IBC:
    // If Hyperlane uses a similar pattern to IBC's IBCStoreStorageSlot:
    // mailboxStorageSlot := common.FromHex("0x...") // From Hyperlane contract
    // return crypto.Keccak256Hash(someKey, paddedSlot)
    
    // The actual implementation would depend on the Hyperlane mailbox contract's
    // storage layout, which can be found by examining the contract source code
    // or using tools like "cast storage" to inspect the storage slots
}

// Alternative: GetMailboxRootStorageSlotWithPath for complex storage layouts
func GetMailboxRootStorageSlotWithPath(path []byte) common.Hash {
    // If the mailbox uses a mapping-based storage like IBC commitments
    mailboxStorageSlot := common.FromHex("0x...") // Base slot for mailbox storage
    
    pathHash := crypto.Keccak256(path)
    paddedSlot := common.LeftPadBytes(mailboxStorageSlot, 32)
    
    // keccak256(h(k) . slot) - same pattern as IBC GetCommitmentsStorageKey
    return crypto.Keccak256Hash(pathHash, paddedSlot)
}
```

## Verification Algorithm

### ZK System Integration Interface

```go
// ZKStateRootVerifier provides an interface to the existing ZK proof system
type ZKStateRootVerifier interface {
    // VerifyStateRoot verifies an EVM state root using a ZK proof
    VerifyStateRoot(stateRoot []byte, blockNumber uint64, zkProof []byte) bool
    
    // IsTrustedStateRoot checks if a state root is in the trusted set
    // (for cases where ZK proof is not immediately available)
    IsTrustedStateRoot(stateRoot MerkleRoot, blockNumber uint64) bool
    
    // GetLatestVerifiedStateRoot returns the most recent ZK-verified state root
    GetLatestVerifiedStateRoot() (MerkleRoot, uint64, error)
}
```

### Combined Verification Function

```go
func VerifyDualMerkleInclusion(
    messageID MessageID,
    metadata DualMerkleInclusionMetadata,
    zkVerifier ZKStateRootVerifier, // Interface to existing ZK system
) error {
    // 1. Verify EVM state root using existing ZK proof system
    if len(metadata.ZKStateProof) > 0 {
        if !zkVerifier.VerifyStateRoot(
            metadata.StateRoot[:],
            metadata.BlockNumber,
            metadata.ZKStateProof,
        ) {
            return fmt.Errorf("ZK state root verification failed")
        }
    } else {
        // Fallback: check if state root is in trusted set
        if !zkVerifier.IsTrustedStateRoot(metadata.StateRoot, metadata.BlockNumber) {
            return fmt.Errorf("state root not trusted and no ZK proof provided")
        }
    }
    
    // 2. Verify mailbox inclusion proof (Message ID → Mailbox Root)
    if !VerifyMerkleProof(
        messageID[:],
        metadata.MessageToMailboxProof,
        metadata.MailboxRoot[:],
    ) {
        return fmt.Errorf("invalid mailbox inclusion proof")
    }
    
    // 3. Verify MPT storage inclusion proof (Mailbox Root → Contract Storage → State Root)  
    if !VerifyMPTStorageProof(
        metadata.MailboxRoot,
        metadata.MailboxStorageProof,
        metadata.MailboxContractAddress,
        metadata.MailboxStorageSlot,
        metadata.StateRoot,
    ) {
        return fmt.Errorf("invalid MPT storage inclusion proof")
    }
    
    return nil // All proofs valid
}
```

### Merkle Proof Verification

```go
func VerifyMerkleProof(
    leaf []byte,
    proof MerkleProof,
    expectedRoot []byte,
) bool {
    // Standard merkle proof verification algorithm
    hash := crypto.Keccak256(leaf)
    index := proof.Index
    
    for _, sibling := range proof.Siblings {
        if index%2 == 0 {
            // Left child - hash(current, sibling)
            hash = crypto.Keccak256(hash, sibling[:])
        } else {
            // Right child - hash(sibling, current)  
            hash = crypto.Keccak256(sibling[:], hash)
        }
        index /= 2
    }
    
    return bytes.Equal(hash, expectedRoot)
}

// VerifyMPTStorageProof verifies that a value exists in contract storage using MPT proofs
func VerifyMPTStorageProof(
    expectedValue MerkleRoot,
    mptProof MPTProof,
    contractAddress common.Address,
    storageKey common.Hash,
    stateRoot MerkleRoot,
) bool {
    // 1. Verify the storage value matches expected value
    expectedBigInt := new(big.Int).SetBytes(expectedValue[:])
    if mptProof.StorageValue.ToInt().Cmp(expectedBigInt) != 0 {
        return false
    }
    
    // 2. Verify the storage key matches
    if mptProof.StorageKey != storageKey {
        return false
    }
    
    // 3. Verify the contract address matches
    if mptProof.Address != contractAddress {
        return false
    }
    
    // 4. Verify storage proof (Storage Key → Storage Hash)
    if !VerifyMPTProof(storageKey[:], mptProof.StorageProof, mptProof.StorageHash[:]) {
        return false
    }
    
    // 5. Verify account proof (Account → State Root)
    accountData := rlpEncodeAccount(
        mptProof.Nonce.Uint64(),
        mptProof.Balance.ToInt(),
        mptProof.StorageHash,
        mptProof.CodeHash,
    )
    accountKey := crypto.Keccak256(contractAddress[:])
    
    return VerifyMPTProof(accountKey, mptProof.AccountProof, stateRoot[:])
}

// VerifyMPTProof verifies a Merkle Patricia Tree proof using go-ethereum's trie package
func VerifyMPTProof(key []byte, proof []hexutil.Bytes, rootHash []byte) bool {
    // Reconstruct the proof database from the proof nodes
    proofDB, err := ReconstructMPTProofDB(proof)
    if err != nil {
        return false
    }
    
    // Use go-ethereum's trie verification (same as celestia-zkevm-ibc-demo/ibc/mpt/mpt.go)
    _, err = trie.VerifyProof(common.BytesToHash(rootHash), key, proofDB)
    return err == nil
}

// VerifyMailboxMPTProof verifies a Hyperlane mailbox MPT proof using the same pattern as IBC
// This is equivalent to VerifyMerklePatriciaTrieProof in celestia-zkevm-ibc-demo/ibc/mpt/mpt.go
func VerifyMailboxMPTProof(
    stateRoot common.Hash, 
    storageKey []byte, 
    proof []hexutil.Bytes,
) (value []byte, err error) {
    // Use the exact same verification logic as the IBC demo
    proofDB, err := ReconstructMPTProofDB(proof)
    if err != nil {
        return nil, fmt.Errorf("failed to decode proof: %w", err)
    }
    return trie.VerifyProof(stateRoot, storageKey, proofDB)
}

// ReconstructMPTProofDB reconstructs a trie database from MPT proof nodes
// Based on celestia-zkevm-ibc-demo/ibc/mpt/mpt.go
func ReconstructMPTProofDB(proof []hexutil.Bytes) (ethdb.Database, error) {
    proofDB := rawdb.NewMemoryDatabase()
    for i, encodedNode := range proof {
        nodeKey := encodedNode
        if len(encodedNode) >= 32 { // small MPT nodes are not hashed
            nodeKey = crypto.Keccak256(encodedNode)
        }
        if err := proofDB.Put(nodeKey, encodedNode); err != nil {
            return nil, fmt.Errorf("failed to load proof node %d into mem db: %w", i, err)
        }
    }
    return proofDB, nil
}

// rlpEncodeAccount RLP encodes account data for verification
func rlpEncodeAccount(nonce uint64, balance *big.Int, storageHash, codeHash common.Hash) []byte {
    // This is a simplified version - actual RLP encoding would be more complex
    // In practice, you'd use go-ethereum's types.StateAccount and RLP encoding
    data := []interface{}{nonce, balance, storageHash, codeHash}
    encoded, _ := rlp.EncodeToBytes(data)
    return encoded
}
```

## Integration Points

### Relayer Implementation

```go
// RelayMessage generates proofs and submits to Celestia
func (r *Relayer) RelayMessage(
    messageID MessageID,
    hyperlaneMessage HyperlaneMessage,
    sourceBlockNumber uint64,
) error {
    // 1. Generate dual inclusion proofs (merkle + MPT)
    metadata, err := r.GenerateDualMerkleMetadata(messageID, sourceBlockNumber)
    if err != nil {
        return fmt.Errorf("failed to generate proofs: %w", err)
    }
    
    // 2. Optionally attach ZK state proof from existing system
    if zkProof, err := r.zkSystem.GetStateProofForBlock(sourceBlockNumber); err == nil {
        metadata.ZKStateProof = zkProof
    }
    // Note: ZK proof may be submitted separately by ZK system for efficiency
    
    // 3. Submit to Celestia with metadata
    return r.celestiaClient.ProcessMessage(hyperlaneMessage, metadata)
}

// GenerateDualMerkleMetadata generates both mailbox and storage proofs
// Following the pattern from celestia-zkevm-ibc-demo
func (r *Relayer) GenerateDualMerkleMetadata(
    messageID MessageID,
    blockNumber uint64,
) (DualMerkleInclusionMetadata, error) {
    // 1. Generate mailbox inclusion proof (standard merkle tree)
    mailboxProof, mailboxRoot, err := GenerateMailboxInclusionProof(
        messageID,
        r.mailboxContract,
        blockNumber,
    )
    if err != nil {
        return DualMerkleInclusionMetadata{}, fmt.Errorf("failed to generate mailbox proof: %w", err)
    }
    
    // 2. Generate MPT storage proof (following getMPTProof pattern)
    mptProof, err := getMailboxMPTProof(
        mailboxRoot,
        r.mailboxAddress.Hex(), // Convert to string like in IBC demo
        blockNumber,
    )
    if err != nil {
        return DualMerkleInclusionMetadata{}, fmt.Errorf("failed to generate MPT proof: %w", err)
    }
    
    // 3. Get state root and block hash from EVM client
    header, err := r.ethClient.HeaderByNumber(
        context.Background(),
        big.NewInt(int64(blockNumber)),
    )
    if err != nil {
        return DualMerkleInclusionMetadata{}, fmt.Errorf("failed to get block header: %w", err)
    }
    
    // 4. Get the storage slot used (from the MPT proof)
    storageSlot := GetMailboxRootStorageSlot()
    
    return DualMerkleInclusionMetadata{
        MessageToMailboxProof:   mailboxProof,
        MailboxStorageProof:     mptProof,
        MailboxRoot:             mailboxRoot,
        MailboxContractAddress:  r.mailboxAddress,
        MailboxStorageSlot:      storageSlot,
        StateRoot:               MerkleRoot(header.Root),
        BlockNumber:             blockNumber,
        BlockHash:               header.Hash(),
    }, nil
}
```

### Celestia Verification Module

```go
// ProcessMessage verifies proofs and processes the message
func (ism *MerkleInclusionISM) ProcessMessage(
    message HyperlaneMessage,
    metadata []byte,
) error {
    // 1. Decode metadata
    var proofMetadata DualMerkleInclusionMetadata
    if err := json.Unmarshal(metadata, &proofMetadata); err != nil {
        return fmt.Errorf("invalid metadata format: %w", err)
    }
    
    // 2. Verify dual inclusion proof using ZK verifier
    messageID := message.ID()
    if err := VerifyDualMerkleInclusion(messageID, proofMetadata, ism.zkVerifier); err != nil {
        return fmt.Errorf("proof verification failed: %w", err)
    }
    
    // 3. Process the verified message
    return ism.processVerifiedMessage(message)
}
```

## Security Considerations

### Trust Assumptions

1. **ZK System Integrity**: The existing ZK proof system correctly verifies EVM state roots
2. **Block Finality**: Only use proofs from finalized blocks to prevent reorganization attacks  
3. **Proof Freshness**: Implement reasonable time bounds on proof age
4. **Merkle Tree Implementation**: The Hyperlane mailbox correctly maintains its merkle tree

### Attack Vectors

1. **Stale Proofs**: Reject proofs older than a maximum age threshold
2. **ZK System Bypass**: Ensure fallback trusted state roots are properly maintained
3. **Malformed Proofs**: Validate all proof structure before verification  
4. **Merkle Tree Manipulation**: Verify mailbox contract integrity and upgrade security

### Recommended Parameters

```go
const (
    MaxProofAge = 24 * time.Hour  // Maximum age for accepted proofs
    MinConfirmations = 12         // Minimum confirmations before trusting state root
    ProofCacheSize = 1000        // LRU cache size for verified proofs
)
```

## Future Extensions

### 1. Enhanced ZK Integration

Optimize integration with the existing ZK proof system:
- Batched ZK state proof verification for multiple blocks
- Asynchronous ZK proof submission and caching
- ZK proof compression for reduced metadata size

### 2. Batch Verification

Optimize for multiple messages:
```go
type BatchMerkleMetadata struct {
    Messages []MessageID
    BatchProof MerkleProof  // Single proof for multiple messages
    StateProof MerkleProof
}
```

### 3. Storage Optimization

Compress proofs for gas efficiency:
```go
type CompressedProof struct {
    Bitmap uint256        // Which siblings are included
    Siblings []byte       // Compressed sibling data
}
```

## Implementation Timeline

### Phase 1: Core Implementation
- [ ] Implement basic data structures (following IBC demo patterns)
- [ ] Add merkle proof generation for mailbox trees
- [ ] Create MPT verification functions (reuse mpt.go patterns)
- [ ] Determine actual Hyperlane mailbox storage slots
- [ ] Unit tests for all components

### Phase 2: Integration
- [ ] Integrate with existing ZK proof system
- [ ] Implement relayer proof generation (following getMPTProof pattern)
- [ ] Add Celestia verification module
- [ ] End-to-end testing with real Hyperlane contracts
- [ ] Performance optimization

### Phase 3: Production Hardening
- [ ] Security audit
- [ ] Comprehensive testing against various Hyperlane versions
- [ ] Integration testing with celestia-zkevm-hl-testnet
- [ ] Monitoring and alerting
- [ ] Documentation and deployment guides

### Phase 4: Deployment Integration
- [ ] Replace NoopISM with MerkleInclusionISM in celestia-zkevm-hl-testnet
- [ ] Update docker-compose and deployment scripts
- [ ] Coordinate with ZK proof system deployment
- [ ] Production rollout and monitoring

## Conclusion

This dual merkle inclusion proof system provides complete cryptographic verification when combined with the existing ZK proof system for EVM state roots. It offers:

- **Full Security**: Cryptographically verifies both state correctness (via ZK) and message inclusion (via merkle proofs)
- **Optimal Performance**: Uses efficient merkle proofs for message verification while leveraging existing ZK infrastructure  
- **System Integration**: Seamlessly integrates with the existing ZK proof system without requiring changes to that infrastructure
- **Production Ready**: Provides immediate security improvement over NoopISM with a clear implementation path

The design achieves the optimal balance between security and performance by using each cryptographic primitive for its strengths: ZK proofs for complex state verification, simple merkle proofs for mailbox inclusion, and Merkle Patricia Tree proofs for EVM storage verification.

This approach is directly inspired by and compatible with the proof system used in the celestia-zkevm-ibc-demo repository, ensuring consistency with existing Celestia IBC infrastructure.

## Comparison with IBC Demo Pattern

This specification follows the exact same pattern as the IBC packet commitment proofs:

| Component | IBC Demo (Packet Commitments) | Hyperlane Demo (Message IDs) |
|-----------|-------------------------------|-------------------------------|
| **Proof Target** | Packet commitment exists in IBC store | Message ID exists in mailbox tree |
| **Contract** | ICS26Router (IBC store) | Hyperlane Mailbox |
| **Storage Calculation** | `GetCommitmentsStorageKey(packetPath)` | `GetMailboxRootStorageSlot()` |
| **Function Pattern** | `getMPTProof(packetPath, contract, block)` | `getMailboxMPTProof(mailboxRoot, contract, block)` |
| **Verification** | `VerifyMerklePatriciaTrieProof()` in mpt.go | `VerifyMPTStorageProof()` (same pattern) |
| **Purpose** | Prove packet was committed on EVM | Prove message was dispatched via mailbox |

Both systems prove that a specific commitment/message exists in an EVM contract's storage, which rolls up to the state root that can be verified by ZK proofs. 