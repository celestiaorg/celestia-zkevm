package cmd

import (
	"context"
	"encoding/hex"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/celestiaorg/celestia-app/v6/app/encoding"
	abci "github.com/cometbft/cometbft/abci/types"
	"github.com/cosmos/cosmos-sdk/client/tx"
	"github.com/cosmos/cosmos-sdk/crypto/hd"
	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	"github.com/cosmos/cosmos-sdk/crypto/keys/secp256k1"
	sdk "github.com/cosmos/cosmos-sdk/types"
	txtypes "github.com/cosmos/cosmos-sdk/types/tx"
	"github.com/cosmos/cosmos-sdk/types/tx/signing"
	authtypes "github.com/cosmos/cosmos-sdk/x/auth/types"
	"google.golang.org/grpc"
)

const (
	denom     = "utia"
	feeAmount = 800
	gasLimit  = 200000
)

var (
	mnemonic = getEnvOrDefault("HYP_MNEMONIC", "sphere exhibit essay fancy okay tuna leaf culture elbow drum trip exchange scorpion excuse parent sun make spot chunk mouse tenant shoe hurt scale")
	chainID  = getEnvOrDefault("HYP_CHAIN_ID", "celestia-zkevm-testnet")
)

func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

type Broadcaster struct {
	enc encoding.Config

	authService authtypes.QueryClient
	txService   txtypes.ServiceClient

	address sdk.AccAddress

	kr keyring.Keyring
}

func NewBroadcaster(enc encoding.Config, grpcConn *grpc.ClientConn) *Broadcaster {
	// Recover private key from mnemonic
	secp256k1Derv := hd.Secp256k1.Derive()
	privKey, err := secp256k1Derv(mnemonic, "", hd.CreateHDPath(118, 0, 0).String())
	if err != nil {
		log.Fatalf("failed to derive pk from mnemonic: %v", err)
	}

	pk := secp256k1.PrivKey{Key: privKey}
	signerAddr := sdk.AccAddress(pk.PubKey().Address())

	kr := keyring.NewInMemory(enc.Codec)
	if err := kr.ImportPrivKeyHex(signerAddr.String(), hex.EncodeToString(pk.Bytes()), pk.Type()); err != nil {
		log.Fatalf("key import failed")
	}

	return &Broadcaster{
		enc:         enc,
		authService: authtypes.NewQueryClient(grpcConn),
		txService:   txtypes.NewServiceClient(grpcConn),
		address:     signerAddr,
		kr:          kr,
	}
}

func (b *Broadcaster) BroadcastTx(ctx context.Context, msgs ...sdk.Msg) *sdk.TxResponse {
	accRes, err := b.authService.Account(ctx, &authtypes.QueryAccountRequest{Address: b.address.String()})
	if err != nil {
		log.Fatalf("failed to query account: %v", err)
	}

	var acc authtypes.BaseAccount
	if err := b.enc.Codec.Unmarshal(accRes.Account.Value, &acc); err != nil {
		log.Fatalf("unmarshal account: %v", err)
	}

	txBuilder := b.enc.TxConfig.NewTxBuilder()
	if err := txBuilder.SetMsgs(msgs...); err != nil {
		log.Fatalf("set msgs: %v", err)
	}

	txBuilder.SetGasLimit(gasLimit)
	txBuilder.SetFeeAmount(sdk.NewCoins(sdk.NewInt64Coin(denom, feeAmount)))

	factory := tx.Factory{}.
		WithKeybase(b.kr).
		WithSignMode(signing.SignMode_SIGN_MODE_DIRECT).
		WithTxConfig(b.enc.TxConfig).
		WithChainID(chainID).
		WithAccountNumber(acc.AccountNumber).
		WithSequence(acc.Sequence)

	if err := tx.Sign(ctx, factory, b.address.String(), txBuilder, false); err != nil {
		log.Fatalf("failed to sign tx: %v", err)
	}

	txBytes, err := b.enc.TxConfig.TxEncoder()(txBuilder.GetTx())
	if err != nil {
		log.Fatalf("encode tx: %v", err)
	}

	broadcastTxReq := &txtypes.BroadcastTxRequest{
		Mode:    txtypes.BroadcastMode_BROADCAST_MODE_SYNC,
		TxBytes: txBytes,
	}

	res, err := b.txService.BroadcastTx(ctx, broadcastTxReq)
	if err != nil || res.TxResponse.Code != abci.CodeTypeOK {
		log.Printf("failed response: %v\n", res.TxResponse)
		log.Fatalf("broadcast tx failed: %v", err)
	}

	txResp, err := b.waitForTxResponse(ctx, res.TxResponse.TxHash)
	if err != nil {
		log.Fatalf("broadcast tx failed: %v", err)
	}

	return txResp
}

func (b *Broadcaster) waitForTxResponse(ctx context.Context, hash string) (*sdk.TxResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()

	ticker := time.NewTicker(6 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("timeout exceeded while waiting for tx confirmation: %w", ctx.Err())
		case <-ticker.C:
			res, err := b.txService.GetTx(ctx, &txtypes.GetTxRequest{Hash: hash})
			if err != nil {
				// Assume tx not found yet; treat as retryable
				continue
			}

			if res != nil && res.TxResponse.Height > 0 {
				return res.TxResponse, nil
			}
		}
	}

}
