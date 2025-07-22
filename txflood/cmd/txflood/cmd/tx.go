package cmd

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"log"
	"math/big"
	"math/rand"
	"strings"
	"sync"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
)

func sendTxs(ctx context.Context, accounts []Wallet, totalTxs uint64) error {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return fmt.Errorf("failed to connect to Ethereum client: %w", err)
	}
	defer client.Close()

	// Convert Wallets to ECDSA keys and build address -> key mapping
	keys := make([]*ecdsa.PrivateKey, 0, len(accounts))
	addrToKey := make(map[common.Address]*ecdsa.PrivateKey)
	nonceMap := make(map[common.Address]uint64)

	for _, acc := range accounts {
		key, err := crypto.HexToECDSA(strings.TrimPrefix(acc.PrivateKey, "0x"))
		if err != nil {
			return fmt.Errorf("failed to decode private key: %w", err)
		}
		addr := crypto.PubkeyToAddress(key.PublicKey)

		nonce, err := client.PendingNonceAt(ctx, addr)
		if err != nil {
			return fmt.Errorf("failed to get nonce for %s: %w", addr.Hex(), err)
		}

		keys = append(keys, key)
		addrToKey[addr] = key
		nonceMap[addr] = nonce
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		return fmt.Errorf("failed to get chain ID: %w", err)
	}

	var wg sync.WaitGroup
	for i := range totalTxs {
		fromKey := keys[i%uint64(len(keys))]
		toKey := keys[(i+1)%uint64(len(keys))]

		fromAddr := crypto.PubkeyToAddress(fromKey.PublicKey)
		toAddr := crypto.PubkeyToAddress(toKey.PublicKey)

		nonce := nonceMap[fromAddr]
		nonceMap[fromAddr]++

		wg.Add(1)
		go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
			defer wg.Done()

			tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

			signedTx, err := types.SignTx(tx, types.NewEIP155Signer(chainID), fromKey)
			if err != nil {
				log.Printf("failed to sign tx: %v", err)
				return
			}

			if err := client.SendTransaction(ctx, signedTx); err != nil {
				log.Printf("failed to send tx with nonce %d: %v", nonce, err)
				return
			}

			fmt.Printf("TxHash: %s\n", signedTx.Hash().Hex())
		}(fromKey, toAddr, nonce)
	}

	wg.Wait()
	return nil
}

func sendTxLoop(ctx context.Context, accounts []Wallet) error {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return fmt.Errorf("failed to connect to Ethereum client: %w", err)
	}
	defer client.Close()

	// Convert Wallets to ECDSA keys and build address -> key mapping
	keys := make([]*ecdsa.PrivateKey, 0, len(accounts))
	addrToKey := make(map[common.Address]*ecdsa.PrivateKey)
	nonceMap := make(map[common.Address]uint64)

	for _, acc := range accounts {
		key, err := crypto.HexToECDSA(strings.TrimPrefix(acc.PrivateKey, "0x"))
		if err != nil {
			return fmt.Errorf("failed to decode private key: %w", err)
		}
		addr := crypto.PubkeyToAddress(key.PublicKey)

		nonce, err := client.PendingNonceAt(ctx, addr)
		if err != nil {
			return fmt.Errorf("failed to get nonce for %s: %w", addr.Hex(), err)
		}

		keys = append(keys, key)
		addrToKey[addr] = key
		nonceMap[addr] = nonce
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		return fmt.Errorf("failed to get chain ID: %w", err)
	}

	ticker := time.NewTicker(3 * time.Second)
	for {
		select {
		case <-ctx.Done():
			fmt.Printf("Exiting...\n")
			return nil
		case <-ticker.C:
			// Generate a random number between 1 and 100
			numTxs := rand.Intn(100) + 1 // Intn returns 0 <= n < 100, so add 1 to get 1–100
			fmt.Printf("Sending %d txs...\n", numTxs)

			var wg sync.WaitGroup
			for i := range numTxs {
				fromKey := keys[i%len(keys)]
				toKey := keys[(i+1)%len(keys)]

				fromAddr := crypto.PubkeyToAddress(fromKey.PublicKey)
				toAddr := crypto.PubkeyToAddress(toKey.PublicKey)

				nonce := nonceMap[fromAddr]
				nonceMap[fromAddr]++

				wg.Add(1)
				go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
					defer wg.Done()

					tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

					signedTx, err := types.SignTx(tx, types.NewEIP155Signer(chainID), fromKey)
					if err != nil {
						log.Printf("failed to sign tx: %v", err)
						return
					}

					if err := client.SendTransaction(ctx, signedTx); err != nil {
						log.Printf("failed to send tx with nonce %d: %v", nonce, err)
						return
					}

					fmt.Printf("TxHash: %s\n", signedTx.Hash().Hex())
				}(fromKey, toAddr, nonce)
			}

			wg.Wait()
		}
	}
}
