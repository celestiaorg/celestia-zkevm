package cmd

import (
	"fmt"
	"log"

	"github.com/bcp-innovations/hyperlane-cosmos/util"
	ismtypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/01_interchain_security/types"
	hooktypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/02_post_dispatch/types"
	coretypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/types"
	warptypes "github.com/bcp-innovations/hyperlane-cosmos/x/warp/types"
	zkismtypes "github.com/celestiaorg/celestia-app/v6/x/zkism/types"
	abci "github.com/cometbft/cometbft/abci/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/gogoproto/proto"
)

func parseIsmIDFromZkISMEvents(events []abci.Event) util.HexAddress {
	var ismID util.HexAddress
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&zkismtypes.EventCreateZKExecutionISM{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if ismEvent, ok := event.(*zkismtypes.EventCreateZKExecutionISM); ok {
				fmt.Printf("successfully created zk execution ISM: %s\n", ismEvent)
				ismID = ismEvent.Id
			}
		}
	}

	return ismID
}

func parseIsmIDFromNoopISMEvents(events []abci.Event) util.HexAddress {
	var ismID util.HexAddress
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&ismtypes.EventCreateNoopIsm{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if ismEvent, ok := event.(*ismtypes.EventCreateNoopIsm); ok {
				log.Printf("successfully created Noop ISM: %s\n", ismEvent)
				ismID = ismEvent.IsmId
			}
		}
	}

	return ismID
}

func parseHooksIDFromEvents(events []abci.Event) util.HexAddress {
	var hookID util.HexAddress
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&hooktypes.EventCreateNoopHook{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if hookEvent, ok := event.(*hooktypes.EventCreateNoopHook); ok {
				log.Printf("successfully created NoopHook: %s\n", hookEvent)
				hookID = hookEvent.NoopHookId
			}
		}
	}

	return hookID
}

func parseMailboxIDFromEvents(events []abci.Event) util.HexAddress {
	var mailboxID util.HexAddress
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&coretypes.EventCreateMailbox{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if mailboxEvent, ok := event.(*coretypes.EventCreateMailbox); ok {
				log.Printf("successfully created Mailbox: %s\n", mailboxEvent)
				mailboxID = mailboxEvent.MailboxId
			}
		}
	}

	return mailboxID
}

func parseCollateralTokenIDFromEvents(events []abci.Event) util.HexAddress {
	var tokenID util.HexAddress
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&warptypes.EventCreateCollateralToken{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if tokenEvent, ok := event.(*warptypes.EventCreateCollateralToken); ok {
				log.Printf("successfully created CollateralToken: %s\n", tokenEvent)
				tokenID = tokenEvent.TokenId
			}
		}
	}

	return tokenID
}

func parseReceiverContractFromEvents(events []abci.Event) string {
	var recvContract string
	for _, evt := range events {
		if evt.GetType() == proto.MessageName(&warptypes.EventEnrollRemoteRouter{}) {
			event, err := sdk.ParseTypedEvent(evt)
			if err != nil {
				log.Fatalf("failed to parse typed event: %v", err)
			}

			if enrollEvent, ok := event.(*warptypes.EventEnrollRemoteRouter); ok {
				log.Printf("successfully enrolled remote router: %s\n", enrollEvent)
				recvContract = enrollEvent.ReceiverContract
			}
		}
	}

	return recvContract
}
