package cmd

import (
	"log"
	"os"
	"os/signal"
	"strconv"
	"strings"

	"github.com/ethereum/go-ethereum/crypto"
	"github.com/spf13/cobra"
)

var (
	walletsFile = "wallets.json"
	rpcURL      = "http://localhost:8545"
)

func NewRootCmd() *cobra.Command {
	rootCmd := &cobra.Command{
		Use:   "txflood",
		Short: "A CLI for sending basic EVM transactions for testing",
		Long: `This CLI provides basic transaction sending functionality for EVM nodes.
It can create new accounts, fund them given a faucet account and spam transactions in a round robin format.`,
		Run: func(cmd *cobra.Command, args []string) {
			cmd.Help()
		},
	}

	rootCmd.AddCommand(CreateAccountsCmd())
	rootCmd.AddCommand(FundAccountsCmd())
	rootCmd.AddCommand(SendTxsCmd())
	rootCmd.AddCommand(SendTxsLoopCmd())

	return rootCmd
}

func CreateAccountsCmd() *cobra.Command {
	createAccCmd := &cobra.Command{
		Use:   "create-accounts [num-accounts]",
		Short: "Create new EVM accounts and write the address and private key to file",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			numAccs, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				log.Fatalf("failed to parse number of accounts: %v", err)
			}

			if err := createAccounts(numAccs, walletsFile); err != nil {
				log.Fatalf("failed to create accounts: %v", err)
			}

			cmd.Printf("Successfully created %d accounts\n", numAccs)
		},
	}

	return createAccCmd
}

func FundAccountsCmd() *cobra.Command {
	fundAccCmd := &cobra.Command{
		Use:   "fund-accounts [faucet-key]",
		Short: "Load accounts from JSON and fund them using a faucet account",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				log.Fatalf("failed to load accounts: %v", err)
			}

			faucetKey, err := crypto.HexToECDSA(strings.TrimPrefix(args[0], "0x"))
			if err != nil {
				log.Fatalf("failed to get faucet private key: %v", err)
			}

			if err := fundAccounts(cmd.Context(), faucetKey, accounts); err != nil {
				log.Fatalf("failed to fund accounts: %v", err)
			}

			cmd.Printf("Successfully funded %d accounts\n", len(accounts))
		},
	}

	return fundAccCmd
}

func SendTxsCmd() *cobra.Command {
	sendTxsCmd := &cobra.Command{
		Use:   "send-txs [num-txs]",
		Short: "Load accounts from JSON and send transactions between them in a round-robin format",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				log.Fatalf("failed to load accounts: %v", err)
			}

			numTxs, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				log.Fatalf("failed to parse number of txs: %v", err)
			}

			if err := sendTxs(cmd.Context(), accounts, numTxs); err != nil {
				log.Fatalf("failed to fund accounts: %v", err)
			}

			cmd.Printf("Successfully sent %d transactions\n", numTxs)
		},
	}

	return sendTxsCmd
}

func SendTxsLoopCmd() *cobra.Command {
	sendTxsLoopCmd := &cobra.Command{
		Use:   "send-loop",
		Short: "Load accounts from JSON and send transactions between them in a round-robin format",
		Args:  cobra.NoArgs,
		Run: func(cmd *cobra.Command, args []string) {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				log.Fatalf("failed to load accounts: %v", err)
			}

			ctx, cancel := signal.NotifyContext(cmd.Context(), os.Interrupt)
			defer cancel()

			if err := sendTxLoop(ctx, accounts); err != nil {
				log.Fatalf("failed to fund accounts: %v", err)
			}
		},
	}

	return sendTxsLoopCmd
}
