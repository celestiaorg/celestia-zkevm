// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/HypNativeMinter.sol";

/**
 * @title DeployHypNativeMinter
 * @notice Deployment script for the HypNativeMinter contract
 *
 * Usage:
 *   forge script script/DeployHypNativeMinter.s.sol:DeployHypNativeMinter \
 *     --rpc-url http://localhost:8545 \
 *     --private-key 0x... \
 *     --broadcast
 */
contract DeployHypNativeMinter is Script {
    // Default values - can be overridden via environment variables
    address constant DEFAULT_MAILBOX = 0xb1c938F5BA4B3593377F399e12175e8db0C787Ff;
    address constant DEFAULT_NATIVE_MINTER = 0x0000000000000000000000000000000000000800;
    uint256 constant DEFAULT_SCALE = 1e12; // 6 decimals to 18 decimals
    address constant DEFAULT_OWNER = 0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d;

    function run() external {
        // Get values from environment or use defaults
        address mailbox = vm.envOr("MAILBOX_ADDRESS", DEFAULT_MAILBOX);
        address nativeMinter = vm.envOr("NATIVE_MINTER_ADDRESS", DEFAULT_NATIVE_MINTER);
        uint256 scale = vm.envOr("SCALE", DEFAULT_SCALE);
        address owner = vm.envOr("OWNER_ADDRESS", DEFAULT_OWNER);

        console.log("Deploying HypNativeMinter with:");
        console.log("  Mailbox:", mailbox);
        console.log("  NativeMinter:", nativeMinter);
        console.log("  Scale:", scale);
        console.log("  Owner:", owner);

        vm.startBroadcast();

        HypNativeMinter hypNativeMinter = new HypNativeMinter(
            mailbox,
            nativeMinter,
            scale,
            owner
        );

        console.log("HypNativeMinter deployed at:", address(hypNativeMinter));

        vm.stopBroadcast();

        // Output for easy copy-paste
        console.log("\n=== Deployment Summary ===");
        console.log("HypNativeMinter:", address(hypNativeMinter));
        console.log("\nNext steps:");
        console.log("1. Update genesis to set this address as admin of the native minter precompile");
        console.log("2. Enroll remote router on Celestia side");
        console.log("3. Call enrollRemoteRouter on this contract with Celestia router address");
    }
}
