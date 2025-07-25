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
	accountsFile = "accounts.json"
	rpcURL       = "http://localhost:8545"
)

// NewRootCmd creates a new txflood root command and wires up its subcommands.
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

// CreateAccountsCmd returns the cmd used for account creation.
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

			if err := createAccounts(numAccs, accountsFile); err != nil {
				return fmt.Errorf("failed to create accounts: %v", err)
			}

			cmd.Printf("Successfully created %d accounts to %s\n", numAccs, accountsFile)
			return nil
		},
	}

	return createAccCmd
}

// FundAccountsCmd returns the cmd used for funding accounts.
func FundAccountsCmd() *cobra.Command {
	fundAccCmd := &cobra.Command{
		Use:   "fund-accounts [faucet-key]",
		Short: "Load accounts from JSON and fund them using a faucet account",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cmd.Printf("Loading accounts from %s\n", accountsFile)
			accounts, err := loadAccounts(accountsFile)
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

// SendTxsCmd returns the cmd used for sending a batch of transactions.
func SendTxsCmd() *cobra.Command {
	sendTxsCmd := &cobra.Command{
		Use:   "send-txs [num-txs]",
		Short: "Load accounts from JSON and send N transactions between them in a round-robin format",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cmd.Printf("Loading accounts from %s\n", accountsFile)
			accounts, err := loadAccounts(accountsFile)
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

// SendTxFloodCmd returns the cmd used for repeatedly sending batches of transactions at an interval.
func SendTxFloodCmd() *cobra.Command {
	sendTxsLoopCmd := &cobra.Command{
		Use:   "flood",
		Short: "Load accounts from JSON and send transactions continuously between them in a round-robin format",
		Long: `Load accounts from JSON and send transactions continuously between them in a round-robin format.
This cmd sends a fixed or random number of transactions at a configurable interval. 
Use the --interval and --num-txs flags to configure the frequency and number of transactions to be sent.
If the --randomise flag is used then the num-txs will be used as the upper bound for the number of transactions sent.`,
		Args: cobra.NoArgs,
		RunE: func(cmd *cobra.Command, args []string) error {
			cmd.Printf("Loading accounts from %s\n", accountsFile)
			accounts, err := loadAccounts(accountsFile)
			if err != nil {
				return fmt.Errorf("failed to load accounts: %v", err)
			}

			interval, err := cmd.Flags().GetDuration("interval")
			if err != nil {
				return fmt.Errorf("failed to parse interval duration from flags")
			}

			numTxs, err := cmd.Flags().GetUint64("num-txs")
			if err != nil {
				return fmt.Errorf("failed to parse number of transactions from flags")
			}

			useRand, err := cmd.Flags().GetBool("randomise")
			if err != nil {
				return fmt.Errorf("failed to parse randomised bool from flags")
			}

			ctx, cancel := signal.NotifyContext(cmd.Context(), os.Interrupt)
			defer cancel()

			if err := sendTxFlood(ctx, accounts, interval, int(numTxs), useRand); err != nil {
				return fmt.Errorf("failed to fund accounts: %v", err)
			}

			return nil
		},
	}

	sendTxsLoopCmd.Flags().Duration("interval", 3*time.Second, "Frequency at which transactions are sent to the node.")
	sendTxsLoopCmd.Flags().Uint64("num-txs", 50, "Number of transactions which are sent to the node.")
	sendTxsLoopCmd.Flags().Bool("randomise", false, "If number of transactions should be randomised using num-txs as the upper bound.")

	return sendTxsLoopCmd
}
