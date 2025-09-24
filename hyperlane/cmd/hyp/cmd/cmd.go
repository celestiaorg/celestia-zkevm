package cmd

import (
	"fmt"
	"log"
	"strconv"

	"github.com/bcp-innovations/hyperlane-cosmos/util"
	ismtypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/01_interchain_security/types"
	"github.com/celestiaorg/celestia-app/v6/app"
	"github.com/celestiaorg/celestia-app/v6/app/encoding"
	"github.com/ethereum/go-ethereum/ethclient"
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
	rootCmd.AddCommand(getEnrollRouterCmd())
	return rootCmd
}

func getDeployZKIsmStackCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy-zkism [celestia-grpc] [evm-rpc]",
		Short: "Deploy cosmosnative hyperlane components using a ZKExecutionIsm to a remote service via gRPC",
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

			rpcAddr := args[1]
			client, err := ethclient.Dial(fmt.Sprintf("http://%s", rpcAddr))
			if err != nil {
				log.Fatal(err)
			}

			ismID := SetupZKIsm(ctx, broadcaster, client)
			SetupWithIsm(ctx, broadcaster, ismID)
		},
	}
	return deployCmd
}

func getDeployNoopIsmStackCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy-noopism [celestia-grpc]",
		Short: "Deploy cosmosnative hyperlane components using a NoopIsm to a remote service via gRPC",
		Args:  cobra.ExactArgs(1),
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

			SetupWithIsm(ctx, broadcaster, ismID)
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
