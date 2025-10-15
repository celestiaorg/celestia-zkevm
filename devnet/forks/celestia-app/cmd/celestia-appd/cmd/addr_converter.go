package cmd

import (
	"fmt"

	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/spf13/cobra"
)

// addrConversionCmd returns a command that converts between celestia1xxx and
// celestiavaloper1xxx addresses.
func addrConversionCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "addr-conversion [celestia address]",
		Short: "Convert between celestia1xxx address and validator operator address celestiavaloper1xxx",
		Long:  `Reads a celestia1xxx or celestiavaloper1xxx address and converts it to the other type.`,
		Example: "celestia-appd addr-conversion celestia1grvklux2yjsln7ztk6slv538396qatckqhs86z\n" +
			"celestia-appd addr-conversion celestiavaloper1grvklux2yjsln7ztk6slv538396qatck9gj7vy\n",
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			converted, err := convertAccountValidatorAddress(args[0])
			if err != nil {
				return err
			}
			_, err = cmd.OutOrStdout().Write([]byte(converted + "\n"))
			if err != nil {
				return err
			}
			return nil
		},
	}
	return cmd
}

// convertAccountValidatorAddress converts an account address into a valoper
// address, or a valoper address into an account address.
func convertAccountValidatorAddress(original string) (string, error) {
	if accAddr, err := sdk.AccAddressFromBech32(original); err == nil {
		return sdk.ValAddress(accAddr.Bytes()).String(), nil
	}
	if valAddr, err := sdk.ValAddressFromBech32(original); err == nil {
		return sdk.AccAddress(valAddr.Bytes()).String(), nil
	}
	return "", fmt.Errorf("invalid address: %s", original)
}
