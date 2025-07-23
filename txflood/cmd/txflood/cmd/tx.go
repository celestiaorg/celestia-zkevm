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

type txClient struct {
	*ethclient.Client

	keys     []*ecdsa.PrivateKey
	nonceMap map[common.Address]uint64
	chainID  *big.Int
}

func newTxClient(ctx context.Context, accounts []Wallet) (*txClient, error) {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to Ethereum client: %w", err)
	}

	keys := make([]*ecdsa.PrivateKey, 0, len(accounts))
	nonceMap := make(map[common.Address]uint64)

	for _, acc := range accounts {
		key, err := crypto.HexToECDSA(strings.TrimPrefix(acc.PrivateKey, "0x"))
		if err != nil {
			client.Close()
			return nil, fmt.Errorf("failed to decode private key: %w", err)
		}

		addr := crypto.PubkeyToAddress(key.PublicKey)
		nonce, err := client.PendingNonceAt(ctx, addr)
		if err != nil {
			client.Close()
			return nil, fmt.Errorf("failed to get nonce for %s: %w", addr.Hex(), err)
		}

		keys = append(keys, key)
		nonceMap[addr] = nonce
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		client.Close()
		return nil, fmt.Errorf("failed to get chain ID: %w", err)
	}

	return &txClient{
		Client:   client,
		keys:     keys,
		nonceMap: nonceMap,
		chainID:  chainID,
	}, nil
}

func sendTxs(ctx context.Context, accounts []Wallet, totalTxs uint64) error {
	txClient, err := newTxClient(ctx, accounts)
	if err != nil {
		return err
	}
	defer txClient.Close()

	var wg sync.WaitGroup
	for i := range totalTxs {
		fromKey := txClient.keys[i%uint64(len(txClient.keys))]
		toKey := txClient.keys[(i+1)%uint64(len(txClient.keys))]

		fromAddr := crypto.PubkeyToAddress(fromKey.PublicKey)
		toAddr := crypto.PubkeyToAddress(toKey.PublicKey)

		nonce := txClient.nonceMap[fromAddr]
		txClient.nonceMap[fromAddr]++

		wg.Add(1)
		go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
			defer wg.Done()

			tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

			signedTx, err := types.SignTx(tx, types.NewEIP155Signer(txClient.chainID), fromKey)
			if err != nil {
				log.Printf("failed to sign tx: %v", err)
				return
			}

			if err := txClient.SendTransaction(ctx, signedTx); err != nil {
				log.Printf("failed to send tx with nonce %d: %v", nonce, err)
				return
			}

			fmt.Printf("TxHash: %s\n", signedTx.Hash().Hex())
		}(fromKey, toAddr, nonce)
	}

	wg.Wait()
	return nil
}

func sendTxFlood(ctx context.Context, accounts []Wallet, interval time.Duration, maxTxs int) error {
	txClient, err := newTxClient(ctx, accounts)
	if err != nil {
		return err
	}
	defer txClient.Close()

	ticker := time.NewTicker(interval)
	for {
		select {
		case <-ctx.Done():
			fmt.Printf("\nExiting transactions send loop...\n")
			return nil
		case <-ticker.C:
			numTxs := rand.Intn(maxTxs) + 1
			fmt.Printf("\nSending %d txs...\n", numTxs)

			var wg sync.WaitGroup
			for i := range numTxs {
				fromKey := txClient.keys[i%len(txClient.keys)]
				toKey := txClient.keys[(i+1)%len(txClient.keys)]

				fromAddr := crypto.PubkeyToAddress(fromKey.PublicKey)
				toAddr := crypto.PubkeyToAddress(toKey.PublicKey)

				nonce := txClient.nonceMap[fromAddr]
				txClient.nonceMap[fromAddr]++

				wg.Add(1)
				go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
					defer wg.Done()

					tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

					signedTx, err := types.SignTx(tx, types.NewEIP155Signer(txClient.chainID), fromKey)
					if err != nil {
						log.Printf("failed to sign tx: %v", err)
						return
					}

					if err := txClient.SendTransaction(ctx, signedTx); err != nil {
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
