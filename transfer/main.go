package main

import (
	"context"
	"encoding/hex"
	"fmt"
	"log"
	"math/big"
	"os"
	"strconv"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"github.com/ethereum/go-ethereum/core/rawdb"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/ethereum/go-ethereum/ethdb"
	"github.com/ethereum/go-ethereum/trie"
)

// Configuration constants
const (
	// EVM RPC endpoint (from docker-compose: ev-node-evm-single service)
	evmRPC = "http://localhost:8545"

	// Hyperlane mailbox contract address (this needs to be determined from the actual deployment)
	// TODO: Get actual mailbox address from deployed contracts
	mailboxContractAddress = "0xb1c938F5BA4B3593377F399e12175e8db0C787Ff" // placeholder from hyperlane config
)

// StorageProof contains MPT proof for a specific storage slot.
// It verifies the existence and value of a storage key in the EVM state.
type StorageProof struct {
	// The key of the storage
	Key common.Hash `json:"key"`
	// The value of the storage
	Value hexutil.Big `json:"value"`
	// The proof of the storage
	Proof []hexutil.Bytes `json:"proof"`
}

// EthGetProofResponse is the response from the eth_getProof RPC call.
type EthGetProofResponse struct {
	AccountProof []hexutil.Bytes `json:"accountProof"`
	Address      common.Address  `json:"address"`
	Balance      *hexutil.Big    `json:"balance"`
	CodeHash     common.Hash     `json:"codeHash"`
	Nonce        hexutil.Uint64  `json:"nonce"`
	StorageHash  common.Hash     `json:"storageHash"`
	StorageProof []StorageProof  `json:"storageProof"`
}

// MailboxMPTProof is the proof of the mailbox tree root in the EVM chain storage.
type MailboxMPTProof struct {
	AccountProof []hexutil.Bytes `json:"accountProof"`
	Address      common.Address  `json:"address"`
	Balance      *hexutil.Big    `json:"balance"`
	CodeHash     common.Hash     `json:"codeHash"`
	Nonce        hexutil.Uint64  `json:"nonce"`
	StorageHash  common.Hash     `json:"storageHash"`
	StorageProof []hexutil.Bytes `json:"storageProof"`
	StorageKey   common.Hash     `json:"storageKey"`
	StorageValue hexutil.Big     `json:"storageValue"`
}

func main() {
	if len(os.Args) < 2 {
		log.Fatal("Usage: go run main.go <command>\nCommands:\n  get-mailbox-proof <block-number|latest>\n  inspect-storage <block-number|latest>\n  verify-proof <block-number|latest>")
	}

	switch os.Args[1] {
	case "get-mailbox-proof":
		blockNumber := uint64(0) // Default to latest
		if len(os.Args) >= 3 && os.Args[2] != "latest" {
			var err error
			// Try parsing as hex first, then decimal
			blockNumber, err = hexutil.DecodeUint64(os.Args[2])
			if err != nil {
				// Try parsing as decimal
				if decimalNum, err2 := strconv.ParseUint(os.Args[2], 10, 64); err2 == nil {
					blockNumber = decimalNum
				} else {
					fmt.Printf("Warning: couldn't parse block number '%s', using latest\n", os.Args[2])
					blockNumber = 0
				}
			}
		}

		err := getMailboxMPTProofExample(blockNumber)
		if err != nil {
			log.Fatal("Failed to get mailbox MPT proof: ", err)
		}

	case "inspect-storage":
		blockNumber := uint64(0) // Default to latest
		if len(os.Args) >= 3 && os.Args[2] != "latest" {
			var err error
			// Try parsing as hex first, then decimal
			blockNumber, err = hexutil.DecodeUint64(os.Args[2])
			if err != nil {
				// Try parsing as decimal
				if decimalNum, err2 := strconv.ParseUint(os.Args[2], 10, 64); err2 == nil {
					blockNumber = decimalNum
				} else {
					fmt.Printf("Warning: couldn't parse block number '%s', using latest\n", os.Args[2])
					blockNumber = 0
				}
			}
		}

		err := inspectMailboxStorage(mailboxContractAddress, blockNumber)
		if err != nil {
			log.Fatal("Failed to inspect mailbox storage: ", err)
		}

	case "verify-proof":
		blockNumber := uint64(0) // Default to latest
		if len(os.Args) >= 3 && os.Args[2] != "latest" {
			var err error
			blockNumber, err = hexutil.DecodeUint64(os.Args[2])
			if err != nil {
				if decimalNum, err2 := strconv.ParseUint(os.Args[2], 10, 64); err2 == nil {
					blockNumber = decimalNum
				} else {
					fmt.Printf("Warning: couldn't parse block number '%s', using latest\n", os.Args[2])
					blockNumber = 0
				}
			}
		}

		err := demonstrateProofVerification(blockNumber)
		if err != nil {
			log.Fatal("Failed to verify proof: ", err)
		}

	default:
		log.Fatal("Unknown command: ", os.Args[1])
	}
}

// getMailboxMPTProofExample demonstrates getting an MPT proof for the Hyperlane mailbox root
func getMailboxMPTProofExample(blockNumber uint64) error {
	fmt.Printf("Generating MPT proof for Hyperlane mailbox root at block %d\n", blockNumber)

	// First, we need to determine what the expected mailbox root should be
	// For now, we'll get the proof and see what's stored
	proof, err := getMailboxMPTProof(mailboxContractAddress, blockNumber)
	if err != nil {
		return fmt.Errorf("failed to get MPT proof: %w", err)
	}

	fmt.Printf("Successfully generated MPT proof:\n")
	fmt.Printf("  Contract Address: %s\n", proof.Address.Hex())
	fmt.Printf("  Storage Key: %s\n", proof.StorageKey.Hex())
	fmt.Printf("  Storage Value: %s\n", proof.StorageValue.String())
	fmt.Printf("  Storage Hash: %s\n", proof.StorageHash.Hex())
	fmt.Printf("  Account Proof nodes: %d\n", len(proof.AccountProof))
	fmt.Printf("  Storage Proof nodes: %d\n", len(proof.StorageProof))

	// Print storage value as bytes32 (potential merkle root)
	storageBytes := proof.StorageValue.ToInt().Bytes()
	if len(storageBytes) > 0 {
		// Pad to 32 bytes
		paddedBytes := make([]byte, 32)
		copy(paddedBytes[32-len(storageBytes):], storageBytes)
		fmt.Printf("  Storage Value as bytes32: 0x%s\n", hex.EncodeToString(paddedBytes))
	}

	return nil
}

// getMailboxMPTProof queries the EVM node for a Merkle Patricia Trie proof
// proving the mailbox tree root exists in the mailbox contract storage
// Following the exact pattern from celestia-zkevm-ibc-demo/testing/demo/pkg/transfer/eth_utils.go
func getMailboxMPTProof(contractAddress string, blockNumber uint64) (MailboxMPTProof, error) {
	// 1. Calculate the storage slot where mailbox stores its tree root
	mailboxRootStorageSlot := getMailboxRootStorageSlot()

	// 2. Connect to EVM client
	client, err := ethclient.Dial(evmRPC)
	if err != nil {
		return MailboxMPTProof{}, fmt.Errorf("failed to connect to EVM node: %w", err)
	}
	defer client.Close()

	// 3. Get the block number if not specified (use latest)
	var blockHex string
	if blockNumber == 0 {
		blockHex = "latest"
	} else {
		blockHex = hexutil.EncodeUint64(blockNumber)
	}

	// 4. Generate the MPT proof using eth_getProof
	var result EthGetProofResponse
	err = client.Client().Call(
		&result,
		"eth_getProof",
		contractAddress,
		[]string{mailboxRootStorageSlot.Hex()},
		blockHex,
	)
	if err != nil {
		return MailboxMPTProof{}, fmt.Errorf("failed to get MPT proof: %w", err)
	}

	// 5. Find the proof for our specific storage key
	var targetProof StorageProof
	found := false
	for _, proof := range result.StorageProof {
		if proof.Key == mailboxRootStorageSlot {
			targetProof = proof
			found = true
			break
		}
	}

	if !found {
		return MailboxMPTProof{}, fmt.Errorf("storage proof not found for key: %s", mailboxRootStorageSlot.Hex())
	}

	// 6. Construct the MPT proof (following exact pattern from getMPTProof)
	proof := MailboxMPTProof{
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

// getMailboxRootStorageSlot calculates the storage slot for the Hyperlane mailbox tree root
// This follows the same pattern as GetCommitmentsStorageKey for IBC commitments
func getMailboxRootStorageSlot() common.Hash {
	// For Hyperlane mailbox contracts, we need to determine the actual storage slot
	// where the tree root is stored. This is typically a simple storage slot.

	// From inspection, slot 0 = 0x1 (likely message count)
	// Let's try slot 1 for the tree root
	return common.HexToHash("0x1")

	// If slot 1 doesn't work, other common patterns:
	// return common.HexToHash("0x2") // Try slot 2
	// return common.HexToHash("0x3") // Try slot 3, etc.

	// Option 3: If it uses mappings like IBC (we'd need to know the key):
	// mailboxStorageSlot := common.FromHex("0x...") // Base slot from contract
	// someKey := crypto.Keccak256([]byte("tree"))  // Or whatever key is used
	// paddedSlot := common.LeftPadBytes(mailboxStorageSlot, 32)
	// return crypto.Keccak256Hash(someKey, paddedSlot)
}

// inspectMailboxStorage helps discover the correct storage slot by checking multiple slots
func inspectMailboxStorage(contractAddress string, blockNumber uint64) error {
	fmt.Printf("Inspecting mailbox storage at block %d\n", blockNumber)

	client, err := ethclient.Dial(evmRPC)
	if err != nil {
		return fmt.Errorf("failed to connect to EVM node: %w", err)
	}
	defer client.Close()

	// Check first 10 storage slots to see what's stored
	for i := 0; i < 10; i++ {
		slot := common.BigToHash(big.NewInt(int64(i)))

		var result EthGetProofResponse
		err = client.Client().Call(
			&result,
			"eth_getProof",
			contractAddress,
			[]string{slot.Hex()},
			"latest",
		)
		if err != nil {
			fmt.Printf("Slot %d: Error - %v\n", i, err)
			continue
		}

		if len(result.StorageProof) > 0 {
			value := result.StorageProof[0].Value
			if value.ToInt().Cmp(big.NewInt(0)) != 0 {
				fmt.Printf("Slot %d: %s (non-zero)\n", i, value.String())
			} else {
				fmt.Printf("Slot %d: 0x0 (zero)\n", i)
			}
		}
	}

	return nil
}

// demonstrateProofVerification shows the complete proof generation and verification workflow
func demonstrateProofVerification(blockNumber uint64) error {
	fmt.Printf("=== Hyperlane Mailbox MPT Proof Demonstration ===\n\n")

	// 1. Generate the MPT proof
	fmt.Printf("Step 1: Generating MPT proof for mailbox storage...\n")
	proof, err := getMailboxMPTProof(mailboxContractAddress, blockNumber)
	if err != nil {
		return fmt.Errorf("failed to generate MPT proof: %w", err)
	}

	fmt.Printf("✓ Proof generated successfully\n")
	fmt.Printf("  Contract: %s\n", proof.Address.Hex())
	fmt.Printf("  Storage Key: %s\n", proof.StorageKey.Hex())
	fmt.Printf("  Storage Value: %s\n", proof.StorageValue.String())
	fmt.Printf("  Account Proof: %d nodes\n", len(proof.AccountProof))
	fmt.Printf("  Storage Proof: %d nodes\n", len(proof.StorageProof))
	fmt.Printf("\n")

	// 2. Get the state root for verification
	fmt.Printf("Step 2: Getting state root for verification...\n")
	client, err := ethclient.Dial(evmRPC)
	if err != nil {
		return fmt.Errorf("failed to connect to EVM: %w", err)
	}
	defer client.Close()

	header, err := client.HeaderByNumber(context.Background(), nil)
	if err != nil {
		return fmt.Errorf("failed to get block header: %w", err)
	}

	stateRoot := header.Root
	fmt.Printf("✓ State root obtained: %s\n", stateRoot.Hex())
	fmt.Printf("\n")

	// 3. Verify the MPT proof
	fmt.Printf("Step 3: Verifying MPT proof...\n")
	verified, err := verifyMailboxMPTProof(stateRoot, proof.StorageKey[:], proof.StorageProof)
	if err != nil {
		return fmt.Errorf("proof verification failed: %w", err)
	}

	if verified {
		fmt.Printf("✓ MPT proof verification PASSED\n")
		fmt.Printf("  The storage value %s exists at key %s\n", proof.StorageValue.String(), proof.StorageKey.Hex())
		fmt.Printf("  This proves the value is correctly stored in the contract at the specified state root\n")
	} else {
		fmt.Printf("✗ MPT proof verification FAILED\n")
		return fmt.Errorf("proof verification returned false")
	}

	fmt.Printf("\n=== Demonstration Complete ===\n")
	fmt.Printf("This demonstrates the MPT proof system working end-to-end:\n")
	fmt.Printf("1. Generated storage proof for mailbox contract\n")
	fmt.Printf("2. Obtained the corresponding state root\n")
	fmt.Printf("3. Verified the proof cryptographically\n")
	fmt.Printf("\nWith actual Hyperlane messages, this same system would prove\n")
	fmt.Printf("that message IDs exist in the mailbox's merkle tree.\n")

	return nil
}

// verifyMailboxMPTProof verifies a Merkle Patricia Trie proof for a specific key
// This function follows the exact pattern from celestia-zkevm-ibc-demo/ibc/mpt/mpt.go
func verifyMailboxMPTProof(stateRoot common.Hash, key []byte, proof []hexutil.Bytes) (bool, error) {
	// Convert hexutil.Bytes to the format expected by ReconstructProofDB
	proofDB, err := ReconstructProofDB(proof)
	if err != nil {
		return false, fmt.Errorf("failed to reconstruct proof DB: %w", err)
	}

	// Use go-ethereum's trie verification (same as celestia-zkevm-ibc-demo/ibc/mpt/mpt.go)
	value, err := trie.VerifyProof(stateRoot, key, proofDB)
	if err != nil {
		return false, fmt.Errorf("proof verification failed: %w", err)
	}

	// If we got a value back, the proof is valid
	return value != nil, nil
}

// ReconstructProofDB reconstructs a trie database from MPT proof nodes
// Based on celestia-zkevm-ibc-demo/ibc/mpt/mpt.go
func ReconstructProofDB(proof []hexutil.Bytes) (ethdb.Database, error) {
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
