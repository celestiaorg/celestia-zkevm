package cmd

import (
	"fmt"
	"log"
	"strconv"
	"strings"

	"github.com/bcp-innovations/hyperlane-cosmos/util"
	ismtypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/01_interchain_security/types"
	coretypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/types"
	warptypes "github.com/bcp-innovations/hyperlane-cosmos/x/warp/types"
	"github.com/celestiaorg/celestia-app/v6/app"
	"github.com/celestiaorg/celestia-app/v6/app/encoding"
	"github.com/ethereum/go-ethereum/ethclient"
	evclient "github.com/evstack/ev-node/pkg/rpc/client"
	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type HyperlaneConfig struct {
	IsmID     util.HexAddress `json:"ism_id"`
	MailboxID util.HexAddress `json:"mailbox_id"`
	HooksID   util.HexAddress `json:"hooks_id"`
	TokenID   util.HexAddress `json:"collateral_token_id"`
}

func NewRootCmd() *cobra.Command {
	rootCmd := &cobra.Command{
		Use:   "hyp",
		Short: "A CLI for deploying hyperlane cosmosnative infrastructure",
		Long: `This CLI provides deployment functionality for hyperlane comosnative modules. 
		It deploys basic core components and warp route collateral token for testing purposes.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			return cmd.Help()
		},
	}

	rootCmd.AddCommand(getDeployNoopIsmStackCmd())
	rootCmd.AddCommand(getDeployZKIsmStackCmd())
	rootCmd.AddCommand(getDeployMultisigIsmStackCmd())
	rootCmd.AddCommand(getEnrollRouterCmd())
	rootCmd.AddCommand(getSetupZkIsmCmd())
	rootCmd.AddCommand(getAnnounceValidatorCmd())
	return rootCmd
}

func getDeployZKIsmStackCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy-zkism [celestia-grpc] [evm-rpc] [ev-node-rpc] [local-domain]",
		Short: "Deploy cosmosnative hyperlane components using a ZKExecutionIsm to a remote service via gRPC",
		Args:  cobra.ExactArgs(4),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)

			evmRpcAddr := args[1]
			client, err := ethclient.Dial(fmt.Sprintf("http://%s", evmRpcAddr))
			if err != nil {
				log.Fatal(err)
			}

			evnodeRpcAddr := args[2]
			evnode := evclient.NewClient(fmt.Sprintf("http://%s", evnodeRpcAddr))

			// Parse local domain
			localDomain, err := strconv.ParseUint(args[3], 10, 32)
			if err != nil {
				log.Fatalf("failed to parse local-domain: %v", err)
			}

			ismID := SetupZKIsm(ctx, broadcaster, client, evnode)
			SetupWithIsm(ctx, broadcaster, ismID, false, uint32(localDomain))
		},
	}

	return deployCmd
}

func getDeployNoopIsmStackCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy-noopism [celestia-grpc] [local-domain]",
		Short: "Deploy cosmosnative hyperlane components using a NoopIsm to a remote service via gRPC",
		Args:  cobra.ExactArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)
			msgCreateNoopISM := ismtypes.MsgCreateNoopIsm{
				Creator: broadcaster.address.String(),
			}

			res := broadcaster.BroadcastTx(ctx, &msgCreateNoopISM)
			ismID := parseIsmIDFromNoopISMEvents(res.Events)

			// Parse local domain
			localDomain, err := strconv.ParseUint(args[1], 10, 32)
			if err != nil {
				log.Fatalf("failed to parse local-domain: %v", err)
			}

			SetupWithIsm(ctx, broadcaster, ismID, false, uint32(localDomain))
		},
	}

	return deployCmd
}

func getDeployMultisigIsmStackCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy-multisigism [celestia-grpc] [validators] [threshold] [local-domain]",
		Short: "Deploy cosmosnative hyperlane components using a MerkleRootMultisigIsm to a remote service via gRPC",
		Long: `Deploy cosmosnative hyperlane components using a MerkleRootMultisigIsm.
		
Validators should be provided as comma-separated ethereum-style addresses (20 bytes).
Example: 0x1234567890123456789012345678901234567890,0xabcdefabcdefabcdefabcdefabcdefabcdefabcd

Threshold is the number of validator signatures required.`,
		Args: cobra.ExactArgs(4),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)

			// Parse validators from comma-separated string
			validatorsStr := args[1]
			validators := []string{}
			if validatorsStr != "" {
				validators = parseValidators(validatorsStr)
			}

			// Parse threshold
			threshold, err := strconv.ParseUint(args[2], 10, 32)
			if err != nil {
				log.Fatalf("failed to parse threshold: %v", err)
			}

			// Validate threshold
			if threshold > uint64(len(validators)) {
				log.Fatalf("threshold (%d) cannot be greater than number of validators (%d)", threshold, len(validators))
			}

			// Parse local domain
			localDomain, err := strconv.ParseUint(args[3], 10, 32)
			if err != nil {
				log.Fatalf("failed to parse local-domain: %v", err)
			}

			msgCreateMultisigISM := ismtypes.MsgCreateMerkleRootMultisigIsm{
				Creator:    broadcaster.address.String(),
				Validators: validators,
				Threshold:  uint32(threshold),
			}

			log.Printf("Creating MerkleRootMultisigIsm with:\n")
			log.Printf("  Creator: %s\n", broadcaster.address.String())
			log.Printf("  Validators: %v\n", validators)
			log.Printf("  Threshold: %d\n", threshold)

			res := broadcaster.BroadcastTx(ctx, &msgCreateMultisigISM)
			log.Printf("Transaction response code: %d\n", res.Code)
			log.Printf("Transaction response log: %s\n", res.RawLog)
			ismID := parseIsmIDFromMultisigISMEvents(res.Events)

			SetupWithIsm(ctx, broadcaster, ismID, true, uint32(localDomain))
		},
	}

	return deployCmd
}

func getEnrollRouterCmd() *cobra.Command {
	enrollRouterCmd := &cobra.Command{
		Use:   "enroll-remote-router [grpc-addr] [token-id] [remote-domain] [remote-contract]",
		Short: "Enroll the remote router contract address for a cosmosnative hyperlane warp route",
		Args:  cobra.ExactArgs(4),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)

			tokenID, err := util.DecodeHexAddress(args[1])
			if err != nil {
				log.Fatalf("failed to parse token id: %v", err)
			}

			domain, err := strconv.ParseUint(args[2], 10, 32)
			if err != nil {
				log.Fatalf("failed to parse remote domain: %v", err)
			}

			receiverContract := args[3]

			SetupRemoteRouter(ctx, broadcaster, tokenID, uint32(domain), receiverContract)
		},
	}
	return enrollRouterCmd
}

func getSetupZkIsmCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "setup-zkism [celestia-grpc] [evm-rpc] [ev-node-rpc]",
		Short: "Deploy a new zk ism and configure it with an existing stack",
		Args:  cobra.ExactArgs(3),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)

			evmRpcAddr := args[1]
			client, err := ethclient.Dial(fmt.Sprintf("http://%s", evmRpcAddr))
			if err != nil {
				log.Fatal(err)
			}

			evnodeRpcAddr := args[2]
			evnode := evclient.NewClient(fmt.Sprintf("http://%s", evnodeRpcAddr))

			ismID := SetupZKIsm(ctx, broadcaster, client, evnode)

			hypQueryClient := coretypes.NewQueryClient(grpcConn)
			mailboxResp, err := hypQueryClient.Mailboxes(ctx, &coretypes.QueryMailboxesRequest{})
			if err != nil {
				log.Fatal(err)
			}

			mailbox := mailboxResp.Mailboxes[0]

			warpQueryClient := warptypes.NewQueryClient(grpcConn)
			tokenResp, err := warpQueryClient.Tokens(ctx, &warptypes.QueryTokensRequest{})
			if err != nil {
				log.Fatal(err)
			}

			token := tokenResp.Tokens[0]

			OverwriteIsm(ctx, broadcaster, ismID, mailbox, token)
		},
	}
	return deployCmd
}

func getAnnounceValidatorCmd() *cobra.Command {
	announceCmd := &cobra.Command{
		Use:   "announce-validator [celestia-grpc] [validator] [storage-location] [signature] [mailbox-id]",
		Short: "Announce a validator signature storage location to the Hyperlane network",
		Long: `Announce a validator to the Hyperlane network by registering its storage location.

Arguments:
  celestia-grpc:     The gRPC address of the Celestia node
  validator:         The validator address (ethereum-style address, with or without 0x prefix)
  storage-location:  The storage location URL where validator signatures can be found
  signature:         The signature proving ownership of the validator address (hex string, with or without 0x prefix)
  mailbox-id:        The mailbox address (ethereum-style address, with or without 0x prefix)

Example:
  hyp announce-validator localhost:9090 0x1234567890123456789012345678901234567890 https://storage.example.com/validator 0xabcdef... 0x9876543210987654321098765432109876543210`,
		Args: cobra.ExactArgs(5),
		Run: func(cmd *cobra.Command, args []string) {
			ctx := cmd.Context()
			enc := encoding.MakeConfig(app.ModuleEncodingRegisters...)

			grpcAddr := args[0]
			grpcConn, err := grpc.NewClient(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Fatalf("failed to connect to gRPC: %v", err)
			}
			defer grpcConn.Close()

			broadcaster := NewBroadcaster(enc, grpcConn)

			// Parse validator address
			validator := args[1]
			if !strings.HasPrefix(validator, "0x") {
				validator = "0x" + validator
			}

			// Storage location
			storageLocation := args[2]

			// Parse signature
			signature := args[3]
			if !strings.HasPrefix(signature, "0x") {
				signature = "0x" + signature
			}

			// Parse mailbox ID
			mailboxIDStr := args[4]
			if !strings.HasPrefix(mailboxIDStr, "0x") {
				mailboxIDStr = "0x" + mailboxIDStr
			}
			mailboxID, err := util.DecodeHexAddress(mailboxIDStr)
			if err != nil {
				log.Fatalf("failed to parse mailbox id: %v", err)
			}

			msgAnnounceValidator := ismtypes.MsgAnnounceValidator{
				Validator:       validator,
				StorageLocation: storageLocation,
				Signature:       signature,
				MailboxId:       mailboxID,
				Creator:         broadcaster.address.String(),
			}

			log.Printf("Announcing validator with:\n")
			log.Printf("  Validator: %s\n", validator)
			log.Printf("  Storage Location: %s\n", storageLocation)
			log.Printf("  Signature: %s\n", signature)
			log.Printf("  Mailbox ID: %s\n", mailboxIDStr)
			log.Printf("  Creator: %s\n", broadcaster.address.String())

			res := broadcaster.BroadcastTx(ctx, &msgAnnounceValidator)
			log.Printf("Transaction response code: %d\n", res.Code)
			log.Printf("Transaction response log: %s\n", res.RawLog)
			log.Printf("Transaction hash: %s\n", res.TxHash)

			if res.Code == 0 {
				log.Printf("✅ Validator announced successfully!\n")
			} else {
				log.Printf("❌ Validator announcement failed\n")
			}
		},
	}
	return announceCmd
}

func parseValidators(validatorsStr string) []string {
	// Split by comma and trim whitespace
	parts := []string{}
	for _, part := range strings.Split(validatorsStr, ",") {
		trimmed := strings.TrimSpace(part)
		if trimmed != "" {
			// Ensure 0x prefix is present
			if !strings.HasPrefix(trimmed, "0x") {
				trimmed = "0x" + trimmed
			}
			parts = append(parts, trimmed)
		}
	}
	return parts
}
