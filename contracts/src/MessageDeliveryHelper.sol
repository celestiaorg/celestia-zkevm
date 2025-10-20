// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/**
 * @title MessageDeliveryHelper
 * @notice Helper contract to manually deliver Hyperlane messages for testing
 * @dev This simulates what the relayer would do
 */
contract MessageDeliveryHelper {
    /**
     * @notice Manually deliver a message to HypNativeMinter
     * @param hypNativeMinter The HypNativeMinter contract address
     * @param origin Origin domain (e.g., 69420 for Celestia)
     * @param sender Sender address as bytes32 (the Celestia router)
     * @param message Message body (recipient + amount)
     */
    function deliverMessage(
        address hypNativeMinter,
        uint32 origin,
        bytes32 sender,
        bytes calldata message
    ) external {
        // Call the handle function on HypNativeMinter
        // This will trigger the minting process
        (bool success, bytes memory returnData) = hypNativeMinter.call(
            abi.encodeWithSignature(
                "handle(uint32,bytes32,bytes)",
                origin,
                sender,
                message
            )
        );

        require(success, string(returnData));
    }
}
