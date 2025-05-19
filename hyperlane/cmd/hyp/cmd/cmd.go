package cmd

import (
	"encoding/json"
	"log"

	"github.com/bcp-innovations/hyperlane-cosmos/util"
	ismtypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/01_interchain_security/types"
	hooktypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/02_post_dispatch/types"
	coretypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/types"
	warptypes "github.com/bcp-innovations/hyperlane-cosmos/x/warp/types"
	"github.com/celestiaorg/celestia-app/v4/app"
	"github.com/celestiaorg/celestia-app/v4/app/encoding"
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
		Run: func(cmd *cobra.Command, args []string) {
			cmd.Help()
		},
	}

	rootCmd.AddCommand(getDeployCmd())
	return rootCmd
}

func getDeployCmd() *cobra.Command {
	deployCmd := &cobra.Command{
		Use:   "deploy [grpc-addr]",
		Short: "Deploy cosmosnative hyperlane components to a remote service via gRPC",
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
			ismID := parseISMFromEvents(res.Events)

			msgCreateNoopHooks := hooktypes.MsgCreateNoopHook{
				Owner: broadcaster.address.String(),
			}

			res = broadcaster.BroadcastTx(ctx, &msgCreateNoopHooks)
			hooksID := parseHooksIDFromEvents(res.Events)

			msgCreateMailBox := coretypes.MsgCreateMailbox{
				Owner:        broadcaster.address.String(),
				DefaultIsm:   ismID,
				LocalDomain:  69420,
				DefaultHook:  &hooksID,
				RequiredHook: &hooksID,
			}

			res = broadcaster.BroadcastTx(ctx, &msgCreateMailBox)
			mailboxID := parseMailboxIDFromEvents(res.Events)

			msgCreateCollateralToken := warptypes.MsgCreateCollateralToken{
				Owner:         broadcaster.address.String(),
				OriginMailbox: mailboxID,
				OriginDenom:   denom,
			}

			res = broadcaster.BroadcastTx(ctx, &msgCreateCollateralToken)
			tokenID := parseCollateralTokenIDFromEvents(res.Events)

			// set ism id on new collateral token (for some reason this can't be done on creation)
			msgSetToken := warptypes.MsgSetToken{
				Owner:    broadcaster.address.String(),
				TokenId:  tokenID,
				IsmId:    &ismID,
				NewOwner: broadcaster.address.String(),
			}

			res = broadcaster.BroadcastTx(ctx, &msgSetToken)

			hypConfig := &HyperlaneConfig{
				IsmID:     ismID,
				HooksID:   hooksID,
				MailboxID: mailboxID,
				TokenID:   tokenID,
			}

			out, err := json.MarshalIndent(hypConfig, "", "  ")
			if err != nil {
				log.Fatalf("failed to marshal config: %v", err)
			}

			cmd.Printf("successfully deployed Hyperlane: \n%s", string(out))
		},
	}
	return deployCmd
}
