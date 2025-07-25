package cmd

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"math/big"
	"math/rand"
	"sync"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/ethclient"
)

type txClient struct {
	*ethclient.Client

	accounts []Account
	chainID  *big.Int
	nonceMap map[common.Address]uint64
}

// newTxClient creates a new EVM transaction client with the provided accounts.
func newTxClient(ctx context.Context, accounts []Account) (*txClient, error) {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to Ethereum client: %w", err)
	}

	nonceMap := make(map[common.Address]uint64)
	for _, acc := range accounts {
		nonce, err := client.PendingNonceAt(ctx, acc.Address)
		if err != nil {
			client.Close()
			return nil, fmt.Errorf("failed to get nonce for %s: %w", acc.Address.Hex(), err)
		}

		nonceMap[acc.Address] = nonce
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		client.Close()
		return nil, fmt.Errorf("failed to get chain ID: %w", err)
	}

	return &txClient{
		Client:   client,
		accounts: accounts,
		nonceMap: nonceMap,
		chainID:  chainID,
	}, nil
}

// sendTxs creates a new tx client and sends totalTxs to the configured EVM node in a round-robin format.
func sendTxs(ctx context.Context, accounts []Account, totalTxs uint64) error {
	txClient, err := newTxClient(ctx, accounts)
	if err != nil {
		return err
	}
	defer txClient.Close()

	var wg sync.WaitGroup
	for i := range totalTxs {
		fromAcc := txClient.accounts[i%uint64(len(txClient.accounts))]
		toAcc := txClient.accounts[(i+1)%uint64(len(txClient.accounts))]

		nonce := txClient.nonceMap[fromAcc.Address]
		txClient.nonceMap[fromAcc.Address]++

		wg.Add(1)
		go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
			defer wg.Done()
			tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

			signedTx, err := types.SignTx(tx, types.NewEIP155Signer(txClient.chainID), fromKey)
			if err != nil {
				fmt.Printf("failed to sign tx: %v", err)
				return
			}

			if err := txClient.SendTransaction(ctx, signedTx); err != nil {
				fmt.Printf("failed to send tx with nonce %d: %v", nonce, err)
				return
			}

			fmt.Printf("TxHash: %s\n", signedTx.Hash().Hex())
		}(fromAcc.PrivateKey, toAcc.Address, nonce)
	}

	wg.Wait()
	return nil
}

// sendTxFlood creates a new ticker at the provided interval and on each tick sends a random number of transactions
// to the configured EVM node capped by the maxTxs upper bound.
func sendTxFlood(ctx context.Context, accounts []Account, interval time.Duration, maxTxs int, randomise bool) error {
	txClient, err := newTxClient(ctx, accounts)
	if err != nil {
		return err
	}
	defer txClient.Close()

	ticker := time.NewTicker(interval)
	fmt.Printf("Starting tx send loop with %s interval\n", interval)

	for {
		select {
		case <-ctx.Done():
			ticker.Stop()
			fmt.Printf("\nExiting transactions send loop...\n")
			return nil
		case <-ticker.C:
			numTxs := rand.Intn(maxTxs) + 1
			fmt.Printf("\nSending %d txs...\n", numTxs)

			var wg sync.WaitGroup
			for i := range numTxs {
				fromAcc := txClient.accounts[i%len(txClient.accounts)]
				toAcc := txClient.accounts[(i+1)%len(txClient.accounts)]

				nonce := txClient.nonceMap[fromAcc.Address]
				txClient.nonceMap[fromAcc.Address]++

				wg.Add(1)
				go func(fromKey *ecdsa.PrivateKey, to common.Address, nonce uint64) {
					defer wg.Done()
					tx := types.NewTransaction(nonce, to, big.NewInt(1e6), 21000, big.NewInt(1e9), nil)

					signedTx, err := types.SignTx(tx, types.NewEIP155Signer(txClient.chainID), fromKey)
					if err != nil {
						fmt.Printf("failed to sign tx: %v", err)
						return
					}

					if err := txClient.SendTransaction(ctx, signedTx); err != nil {
						fmt.Printf("failed to send tx with nonce %d: %v", nonce, err)
						return
					}

					fmt.Printf("TxHash: %s\n", signedTx.Hash().Hex())
				}(fromAcc.PrivateKey, toAcc.Address, nonce)
			}
			wg.Wait()
		}
	}
}
