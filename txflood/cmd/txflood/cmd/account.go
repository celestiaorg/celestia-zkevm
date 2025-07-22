package cmd

import (
	"context"
	"crypto/ecdsa"
	"encoding/json"
	"fmt"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
)

type Wallet struct {
	Address    string `json:"address"`
	PrivateKey string `json:"private_key"`
}

func loadAccounts(inFile string) ([]Wallet, error) {
	data, err := os.ReadFile(inFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read file %s: %w", inFile, err)
	}

	var wallets []Wallet
	if err := json.Unmarshal(data, &wallets); err != nil {
		return nil, fmt.Errorf("failed to unmarshal wallets: %w", err)
	}

	return wallets, nil
}

func createAccounts(count uint64, outFile string) error {
	var wallets []Wallet
	for range count {
		key, err := crypto.GenerateKey()
		if err != nil {
			return err
		}

		address := crypto.PubkeyToAddress(key.PublicKey).Hex()
		privHex := fmt.Sprintf("0x%x", crypto.FromECDSA(key))

		wallets = append(wallets, Wallet{
			Address:    address,
			PrivateKey: privHex,
		})
	}

	data, err := json.MarshalIndent(wallets, "", "  ")
	if err != nil {
		return err
	}

	err = os.WriteFile(outFile, data, 0600)
	if err != nil {
		return err
	}

	return nil
}

func fundAccounts(ctx context.Context, faucetKey *ecdsa.PrivateKey, accounts []Wallet) error {
	client, err := ethclient.Dial(rpcURL)
	if err != nil {
		return fmt.Errorf("failed to connect to the Ethereum client: %v", err)
	}
	defer client.Close()

	faucetAddr := crypto.PubkeyToAddress(faucetKey.PublicKey)
	nonce, err := client.PendingNonceAt(ctx, faucetAddr)
	if err != nil {
		return fmt.Errorf("Failed to get nonce: %v", err)
	}

	// Set a fixed value and gas price for simplicity
	amount := big.NewInt(1e17) // 0.1 ETH

	gasLimit := uint64(21000)
	gasPrice, err := client.SuggestGasPrice(ctx)
	if err != nil {
		return fmt.Errorf("Failed to get gas price: %v", err)
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		return fmt.Errorf("Failed to get chain ID: %v", err)
	}

	for i, acc := range accounts {
		toAddr := common.HexToAddress(acc.Address)
		tx := types.NewTransaction(nonce+uint64(i), toAddr, amount, gasLimit, gasPrice, nil)

		signedTx, err := types.SignTx(tx, types.NewEIP155Signer(chainID), faucetKey)
		if err != nil {
			return fmt.Errorf("Failed to sign tx: %v", err)
		}

		err = client.SendTransaction(ctx, signedTx)
		if err != nil {
			return fmt.Errorf("Failed to send tx to %s: %v", acc.Address, err)
		}
	}

	return nil
}
