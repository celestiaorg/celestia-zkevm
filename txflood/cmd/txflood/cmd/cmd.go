package cmd

import (
	"fmt"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"time"

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
	rootCmd.AddCommand(SendTxFloodCmd())

	return rootCmd
}

func CreateAccountsCmd() *cobra.Command {
	createAccCmd := &cobra.Command{
		Use:   "create-accounts [num-accounts]",
		Short: "Create new EVM accounts and write the address and private key to file",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			numAccs, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				return fmt.Errorf("failed to parse number of accounts: %v", err)
			}

			if err := createAccounts(numAccs, walletsFile); err != nil {
				return fmt.Errorf("failed to create accounts: %v", err)
			}

			cmd.Printf("Successfully created %d accounts\n", numAccs)
			return nil
		},
	}

	return createAccCmd
}

func FundAccountsCmd() *cobra.Command {
	fundAccCmd := &cobra.Command{
		Use:   "fund-accounts [faucet-key]",
		Short: "Load accounts from JSON and fund them using a faucet account",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				return fmt.Errorf("failed to load accounts: %v", err)
			}

			faucetKey, err := crypto.HexToECDSA(strings.TrimPrefix(args[0], "0x"))
			if err != nil {
				return fmt.Errorf("failed to get faucet private key: %v", err)
			}

			if err := fundAccounts(cmd.Context(), faucetKey, accounts); err != nil {
				return fmt.Errorf("failed to fund accounts: %v", err)
			}

			cmd.Printf("Successfully funded %d accounts\n", len(accounts))
			return nil
		},
	}

	return fundAccCmd
}

func SendTxsCmd() *cobra.Command {
	sendTxsCmd := &cobra.Command{
		Use:   "send-txs [num-txs]",
		Short: "Load accounts from JSON and send N transactions between them in a round-robin format",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				return fmt.Errorf("failed to load accounts: %v", err)
			}

			numTxs, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				return fmt.Errorf("failed to parse number of txs: %v", err)
			}

			if err := sendTxs(cmd.Context(), accounts, numTxs); err != nil {
				return fmt.Errorf("failed to fund accounts: %v", err)
			}

			cmd.Printf("Successfully sent %d transactions\n", numTxs)
			return nil
		},
	}

	return sendTxsCmd
}

func SendTxFloodCmd() *cobra.Command {
	sendTxsLoopCmd := &cobra.Command{
		Use:   "flood",
		Short: "Load accounts from JSON and send transactions continuously between them in a round-robin format",
		Long: `Load accounts from JSON and send transactions continuously between them in a round-robin format.
This cmd sends a random number of transactions capped at an upper bound and at a configurable interval. 
Use the --interval and --max-txs flags to configure the frequency and upper bound of transactions to be sent.`,
		Args: cobra.NoArgs,
		RunE: func(cmd *cobra.Command, args []string) error {
			accounts, err := loadAccounts(walletsFile)
			if err != nil {
				return fmt.Errorf("failed to load accounts: %v", err)
			}

			interval, err := cmd.Flags().GetDuration("interval")
			if err != nil {
				return fmt.Errorf("failed to parse interval duration from flags")
			}

			maxTxs, err := cmd.Flags().GetUint64("max-txs")
			if err != nil {
				return fmt.Errorf("failed to parse max transactions from flags")
			}

			ctx, cancel := signal.NotifyContext(cmd.Context(), os.Interrupt)
			defer cancel()

			if err := sendTxFlood(ctx, accounts, interval, int(maxTxs)); err != nil {
				return fmt.Errorf("failed to fund accounts: %v", err)
			}

			return nil
		},
	}

	sendTxsLoopCmd.Flags().Duration("interval", 3*time.Second, "Frequency at which transactions are sent to the node.")
	sendTxsLoopCmd.Flags().Uint64("max-txs", 100, "Frequency at which transactions are sent to the node.")

	return sendTxsLoopCmd
}
