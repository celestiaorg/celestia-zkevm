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
	rpcclient "github.com/cometbft/cometbft/rpc/client/http"
	"github.com/ethereum/go-ethereum/ethclient"
	evclient "github.com/evstack/ev-node/pkg/rpc/client"
)

const (
	// Currently we hardcode this value here as this is the canonical namespace used by the
	// infrastructure in this repo.
	namespaceHex = "00000000000000000000000000000000000000a8045f161bf468bf4d44"
)

// SetupZkIsm deploys a new zk ism using the provided evm client to fetch the latest block
// for the initial trusted height and trusted root.
func SetupZKIsm(ctx context.Context, broadcaster *Broadcaster, ethClient *ethclient.Client, evnodeClient *evclient.Client) util.HexAddress {
	block, err := ethClient.BlockByNumber(ctx, nil) // nil == latest
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("successfully got block %d from ev-reth\n", block.NumberU64())

	namespace, err := hex.DecodeString(namespaceHex)
	if err != nil {
		log.Fatal(err)
	}

	pubKey, err := getSequencerPubKey(ctx, evnodeClient)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("successfully got pubkey from ev-node %x\n", pubKey)

	groth16Vkey := readGroth16Vkey()
	stateTransitionVkey := readStateTransitionVkey()
	stateMembershipVkey := readStateMembershipVkey()

	root, height := GetCelestiaBlockHashAndHeight(ctx, "http://celestia-validator:26657")

	fmt.Printf("successfully got celestia root and height: %x, %d\n", root, height)

	msgCreateZkExecutionISM := zkismtypes.MsgCreateZKExecutionISM{
		Creator:             broadcaster.address.String(),
		StateRoot:           block.Header().Root.Bytes(),
		Height:              block.NumberU64(),
		CelestiaHeaderHash:  root[:],
		CelestiaHeight:      height,
		Namespace:           namespace,
		SequencerPublicKey:  pubKey,
		Groth16Vkey:         groth16Vkey,
		StateTransitionVkey: stateTransitionVkey,
		StateMembershipVkey: stateMembershipVkey,
	}

	res := broadcaster.BroadcastTx(ctx, &msgCreateZkExecutionISM)

	ismID := parseIsmIDFromZkISMEvents(res.Events)

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

func OverwriteIsm(ctx context.Context, broadcaster *Broadcaster, ismID util.HexAddress, mailbox coretypes.Mailbox, token warptypes.WrappedHypToken) {
	msgSetMailbox := coretypes.MsgSetMailbox{
		Owner:             broadcaster.address.String(),
		MailboxId:         mailbox.Id,
		DefaultIsm:        &ismID,
		RenounceOwnership: false,
	}

	tokenID, err := util.DecodeHexAddress(token.Id)
	if err != nil {
		log.Fatal(err)
	}

	// set ism id on new collateral token (for some reason this can't be done on creation)
	msgSetToken := warptypes.MsgSetToken{
		Owner:    broadcaster.address.String(),
		TokenId:  tokenID,
		IsmId:    &ismID,
		NewOwner: broadcaster.address.String(),
	}

	broadcaster.BroadcastTx(ctx, &msgSetMailbox, &msgSetToken)

	cfg := &HyperlaneConfig{
		IsmID:     ismID,
		HooksID:   *mailbox.RequiredHook,
		MailboxID: mailbox.Id,
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

func getSequencerPubKey(ctx context.Context, client *evclient.Client) ([]byte, error) {
	resp, err := client.GetBlockByHeight(ctx, 1)
	if err != nil {
		return nil, err
	}

	return resp.Block.Header.Signer.PubKey[4:], nil
}

func readGroth16Vkey() []byte {
	groth16Vkey, err := os.ReadFile("testdata/vkeys/groth16_vk.bin")
	if err != nil {
		log.Fatal(err)
	}

	return groth16Vkey
}

func readStateTransitionVkey() []byte {
	data, err := os.ReadFile("testdata/vkeys/ev-range-exec-vkey-hash")
	if err != nil {
		log.Fatal(err)
	}

	hashStr := strings.TrimSpace(string(data))
	hashBz, err := hex.DecodeString(strings.TrimPrefix(hashStr, "0x"))
	if err != nil {
		log.Fatalf("failed to decode hex: %v", err)
	}

	return hashBz
}

func readStateMembershipVkey() []byte {
	data, err := os.ReadFile("testdata/vkeys/ev-hyperlane-vkey-hash")
	if err != nil {
		log.Fatal(err)
	}

	hashStr := strings.TrimSpace(string(data))
	hashBz, err := hex.DecodeString(strings.TrimPrefix(hashStr, "0x"))
	if err != nil {
		log.Fatalf("failed to decode hex: %v", err)
	}

	return hashBz
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

func GetCelestiaBlockHashAndHeight(ctx context.Context, rpcAddr string) ([32]byte, uint64) {
	client, err := rpcclient.New(rpcAddr, "/websocket")
	if err != nil {
		log.Fatalf("failed to connect to Celestia RPC: %v", err)
	}
	defer client.Stop()

	status, err := client.Status(ctx)
	if err != nil {
		log.Fatalf("failed to get Celestia status: %v", err)
	}

	height := uint64(status.SyncInfo.LatestBlockHeight)
	heightInt64 := int64(height)

	block, err := client.Block(ctx, &heightInt64)
	if err != nil {
		log.Fatalf("failed to fetch block at height %d: %v", height, err)
	}

	blockHash := block.BlockID.Hash.Bytes()

	var hash [32]byte
	if len(blockHash) != 32 {
		log.Fatalf("unexpected block hash length: %d", len(blockHash))
	}
	copy(hash[:], blockHash)

	fmt.Printf("Celestia node height: %d\nBlock header hash: 0x%s\n",
		height, hex.EncodeToString(hash[:]))

	return hash, height
}
