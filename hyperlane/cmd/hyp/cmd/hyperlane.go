package cmd

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"strings"

	"cosmossdk.io/math"
	"github.com/bcp-innovations/hyperlane-cosmos/util"
	hooktypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/02_post_dispatch/types"
	coretypes "github.com/bcp-innovations/hyperlane-cosmos/x/core/types"
	warptypes "github.com/bcp-innovations/hyperlane-cosmos/x/warp/types"
	zkismtypes "github.com/celestiaorg/celestia-app/v6/x/zkism/types"
	"github.com/ethereum/go-ethereum/ethclient"
)

var (
	// TODO: Configure these values either by arguments or environment for convenience
	stateVkeyHash   = "0x00acd6f9c9d0074611353a1e0c94751d3c49beef64ebc3ee82f0ddeadaf242ef"
	messageVkeyHash = "0x00c88cdad907c05533b8755953d58af6a3b753a4e05acc6617d41ca206c25d2a"
	namespaceHex    = "00000000000000000000000000000000000000a8045f161bf468bf4d44"
	publicKeyHex    = "c87f6c4cdd4c8ac26cb6a06909e5e252b73043fdf85232c18ae92b9922b65507"
)

// SetupZkIsm deploys a new zk ism using the provided evm client to fetch the latest block
// for the initial trusted height and trusted root.
func SetupZKIsm(ctx context.Context, broadcaster *Broadcaster, ethClient *ethclient.Client) util.HexAddress {
	block, err := ethClient.BlockByNumber(ctx, nil) // nil == latest
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("successfully got block %d from ev-reth\n", block.NumberU64())

	stateVkeyHex := strings.TrimPrefix(stateVkeyHash, "0x")
	stateVkey, err := hex.DecodeString(stateVkeyHex)
	if err != nil {
		log.Fatal(err)
	}

	messageVkeyHex := strings.TrimPrefix(messageVkeyHash, "0x")
	messageVkey, err := hex.DecodeString(messageVkeyHex)
	if err != nil {
		log.Fatal(err)
	}

	namespace, err := hex.DecodeString(namespaceHex)
	if err != nil {
		log.Fatal(err)
	}

	pubKey, err := hex.DecodeString(publicKeyHex)
	if err != nil {
		log.Fatal(err)
	}

	groth16Vkey := readGroth16Vkey()

	msgCreateZkExecutionISM := zkismtypes.MsgCreateZKExecutionISM{
		Creator:             broadcaster.address.String(),
		StateRoot:           block.Header().Root.Bytes(),
		Height:              block.NumberU64(),
		Namespace:           namespace,
		SequencerPublicKey:  pubKey,
		Groth16Vkey:         groth16Vkey,
		StateTransitionVkey: stateVkey,
		StateMembershipVkey: messageVkey,
	}

	res := broadcaster.BroadcastTx(ctx, &msgCreateZkExecutionISM)
	ismID := parseIsmIDFromZkISMEvents(res.Events)

	fmt.Printf("successfully created zk execution ism: %s\n", ismID)
	return ismID
}

// SetupWithIsm deploys the cosmosnative Hyperlane components using the provided ism identifier.
func SetupWithIsm(ctx context.Context, broadcaster *Broadcaster, ismID util.HexAddress) {
	msgCreateNoopHooks := hooktypes.MsgCreateNoopHook{
		Owner: broadcaster.address.String(),
	}

	res := broadcaster.BroadcastTx(ctx, &msgCreateNoopHooks)
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

	broadcaster.BroadcastTx(ctx, &msgSetToken)

	cfg := &HyperlaneConfig{
		IsmID:     ismID,
		HooksID:   hooksID,
		MailboxID: mailboxID,
		TokenID:   tokenID,
	}

	writeConfig(cfg)
}

// SetupRemoteRouter links the provided token identifier on the cosmosnative deployment with the receiver contract on the counterparty.
// For example: if the provided token identifier is a collateral token (e.g. utia), the receiverContract is expected to be the
// contract address for the corresponding synthetic token on the counterparty.
func SetupRemoteRouter(ctx context.Context, broadcaster *Broadcaster, tokenID util.HexAddress, domain uint32, receiverContract string) {
	msgEnrollRemoteRouter := warptypes.MsgEnrollRemoteRouter{
		Owner:   broadcaster.address.String(),
		TokenId: tokenID,
		RemoteRouter: &warptypes.RemoteRouter{
			ReceiverDomain:   domain,
			ReceiverContract: receiverContract,
			Gas:              math.ZeroInt(),
		},
	}

	res := broadcaster.BroadcastTx(ctx, &msgEnrollRemoteRouter)
	recvContract := parseReceiverContractFromEvents(res.Events)

	fmt.Printf("successfully registered remote router on Hyperlane cosmosnative: \n%s", recvContract)
}

func readGroth16Vkey() []byte {
	groth16Vkey, err := os.ReadFile("testdata/vkeys/groth16_vk.bin")
	if err != nil {
		log.Fatal(err)
	}

	return groth16Vkey
}

func writeConfig(cfg *HyperlaneConfig) {
	out, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		log.Fatalf("failed to marshal config: %v", err)
	}

	outputPath := "hyperlane-cosmosnative.json"
	if err := os.WriteFile(outputPath, out, 0o644); err != nil {
		log.Fatalf("failed to write JSON file: %v", err)
	}

	fmt.Printf("successfully deployed Hyperlane: \n%s\n", string(out))
}
