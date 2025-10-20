// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/MessageDeliveryHelper.sol";

/**
 * @title DeliverMessage
 * @notice Script to manually deliver a Hyperlane message to test native minting
 */
contract DeliverMessage is Script {
    function run() external {
        // Deployed addresses
        address HYPNATIVEMINTER = 0x81a91d503d2c171d9148827f549e51C286Acc97D;

        // Message parameters
        uint32 CELESTIA_DOMAIN = 69420;
        bytes32 CELESTIA_ROUTER = 0x345a583028762de4d733852c9d4f419077093a48000000000000000000000000;

        // Recipient and amount
        address recipient = 0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d;
        uint256 amount = 10_000_000; // 10 TIA in 6 decimals

        // Build message body: recipient (32 bytes) + amount (32 bytes)
        bytes32 recipientBytes32 = bytes32(uint256(uint160(recipient)));
        bytes memory messageBody = abi.encodePacked(recipientBytes32, amount);

        console.log("=== Manually Delivering Hyperlane Message ===");
        console.log("");
        console.log("HypNativeMinter:", HYPNATIVEMINTER);
        console.log("Origin Domain:", CELESTIA_DOMAIN);
        console.log("Sender (Celestia Router):", vm.toString(CELESTIA_ROUTER));
        console.log("Recipient:", recipient);
        console.log("Amount (6 decimals):", amount);
        console.log("Expected mint (18 decimals):", amount * 1e12);
        console.log("");

        // Check balance before
        uint256 balanceBefore = recipient.balance;
        console.log("Balance BEFORE:", balanceBefore);
        console.log("");

        vm.startBroadcast();

        // Deploy helper
        MessageDeliveryHelper helper = new MessageDeliveryHelper();
        console.log("Helper deployed at:", address(helper));

        // Deliver the message
        console.log("Delivering message...");
        helper.deliverMessage(
            HYPNATIVEMINTER,
            CELESTIA_DOMAIN,
            CELESTIA_ROUTER,
            messageBody
        );

        vm.stopBroadcast();

        // Check balance after
        uint256 balanceAfter = recipient.balance;
        console.log("");
        console.log("Balance AFTER:", balanceAfter);
        console.log("Minted:", balanceAfter - balanceBefore, "wei");
        console.log("");

        if (balanceAfter > balanceBefore) {
            console.log("SUCCESS! Native TIA minted!");
        } else {
            console.log("WARNING: Balance did not increase");
        }
    }
}
