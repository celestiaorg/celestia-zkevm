package cmd

import (
	"context"
	"crypto/ecdsa"
	"encoding/json"
	"fmt"
	"math/big"
	"os"
	"strings"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
)

// Account encapsulates an eth address and private key.
type Account struct {
	Address    common.Address    `json:"-"`
	PrivateKey *ecdsa.PrivateKey `json:"-"`
}

// struct used only for JSON encoding/decoding
type accountJSON struct {
	Address    string `json:"address"`
	PrivateKey string `json:"private_key"`
}

// MarshalJSON serializes Wallet as JSON with hex fields
func (w *Account) MarshalJSON() ([]byte, error) {
	return json.Marshal(accountJSON{
		Address:    w.Address.Hex(),
		PrivateKey: fmt.Sprintf("0x%x", crypto.FromECDSA(w.PrivateKey)),
	})
}

// UnmarshalJSON deserializes hex strings to common.Address and *ecdsa.PrivateKey
func (w *Account) UnmarshalJSON(data []byte) error {
	var accJSON accountJSON
	if err := json.Unmarshal(data, &accJSON); err != nil {
		return err
	}

	pk, err := crypto.HexToECDSA(strings.TrimPrefix(accJSON.PrivateKey, "0x"))
	if err != nil {
		return fmt.Errorf("invalid private key: %w", err)
	}

	w.PrivateKey = pk
	w.Address = crypto.PubkeyToAddress(pk.PublicKey)

	return nil
}

func loadAccounts(inFile string) ([]Account, error) {
	data, err := os.ReadFile(inFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read file %s: %w", inFile, err)
	}

	var accounts []Account
	if err := json.Unmarshal(data, &accounts); err != nil {
		return nil, fmt.Errorf("failed to unmarshal accounts: %w", err)
	}

	return accounts, nil
}

func createAccounts(count uint64, outFile string) error {
	var accounts []*Account
	for i := uint64(0); i < count; i++ {
		key, err := crypto.GenerateKey()
		if err != nil {
			return err
		}

		accounts = append(accounts, &Account{
			PrivateKey: key,
			Address:    crypto.PubkeyToAddress(key.PublicKey),
		})
	}

	data, err := json.MarshalIndent(accounts, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal accounts: %w", err)
	}

	if err := os.WriteFile(outFile, data, 0600); err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	return nil
}

func fundAccounts(ctx context.Context, faucetKey *ecdsa.PrivateKey, accounts []Account) error {
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
		tx := types.NewTransaction(nonce+uint64(i), acc.Address, amount, gasLimit, gasPrice, nil)

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
