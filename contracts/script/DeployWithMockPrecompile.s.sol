// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/HypNativeMinter.sol";
import "../src/MockNativeMinter.sol";

/**
 * @title DeployWithMockPrecompile
 * @notice Deploy HypNativeMinter with a mock precompile for testing with real Hyperlane
 *
 * This script:
 * 1. Deploys MockNativeMinter (simulates the precompile)
 * 2. Deploys HypNativeMinter (your custom router)
 * 3. Sets HypNativeMinter as admin of MockNativeMinter
 * 4. Funds MockNativeMinter so it can "mint" tokens
 *
 * Once the real precompile is ready, you just need to:
 * - Redeploy HypNativeMinter with the real precompile address (0x0800)
 * - Configure the precompile admin in genesis
 *
 * Usage:
 *   # Start your local network with Hyperlane
 *   make start
 *
 *   # Deploy the mock setup
 *   forge script script/DeployWithMockPrecompile.s.sol:DeployWithMockPrecompile \
 *     --rpc-url http://localhost:8545 \
 *     --private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
 *     --broadcast
 */
contract DeployWithMockPrecompile is Script {
    // Your existing Hyperlane mailbox
    address constant MAILBOX = 0xb1c938F5BA4B3593377F399e12175e8db0C787Ff;

    // Default values
    uint256 constant SCALE = 1e12; // 6 decimals to 18 decimals
    address constant OWNER = 0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d;

    // Amount to fund the mock precompile (100 TIA)
    uint256 constant MOCK_FUNDING = 100 ether;

    function run() external {
        console.log("=== Deploying Mock Native Minter Setup ===\n");

        vm.startBroadcast();

        // 1. Deploy MockNativeMinter
        console.log("1. Deploying MockNativeMinter...");
        MockNativeMinter mockPrecompile = new MockNativeMinter();
        console.log("   MockNativeMinter deployed at:", address(mockPrecompile));

        // 2. Deploy HypNativeMinter
        console.log("\n2. Deploying HypNativeMinter...");
        HypNativeMinter hypNativeMinter = new HypNativeMinter(
            MAILBOX,
            address(mockPrecompile),
            SCALE,
            OWNER
        );
        console.log("   HypNativeMinter deployed at:", address(hypNativeMinter));

        // 3. Set HypNativeMinter as admin of MockNativeMinter
        console.log("\n3. Setting HypNativeMinter as admin...");
        mockPrecompile.addAdmin(address(hypNativeMinter));
        console.log("   Admin set successfully");

        // 4. Fund the mock precompile
        console.log("\n4. Funding MockNativeMinter with", MOCK_FUNDING / 1e18, "TIA...");
        (bool sent,) = address(mockPrecompile).call{value: MOCK_FUNDING}("");
        require(sent, "Funding failed");
        console.log("   Funded successfully");

        vm.stopBroadcast();

        // Output summary
        console.log("\n=== Deployment Complete ===");
        console.log("MockNativeMinter:", address(mockPrecompile));
        console.log("HypNativeMinter:", address(hypNativeMinter));
        console.log("Mailbox:", MAILBOX);
        console.log("Owner:", OWNER);
        console.log("Scale:", SCALE);

        console.log("\n=== Next Steps ===");
        console.log("1. Update your warp route config to use HypNativeMinter");
        console.log("2. Deploy/update warp routes on Celestia side");
        console.log("3. Enroll remote routers:");
        console.log("   cast send", address(hypNativeMinter));
        console.log("     'enrollRemoteRouter(uint32,bytes32)'");
        console.log("     69420");  // Celestia domain
        console.log("     <CELESTIA_ROUTER_ADDRESS_AS_BYTES32>");
        console.log("     --private-key <KEY>");
        console.log("     --rpc-url http://localhost:8545");

        console.log("\n4. Test the flow:");
        console.log("   make transfer  # Bridge from Celestia to Eden");
        console.log("   # Native TIA should be minted on Eden!");

        console.log("\n=== When Real Precompile is Ready ===");
        console.log("Just redeploy HypNativeMinter with:");
        console.log("  nativeMinter = 0x0000000000000000000000000000000000000800");
        console.log("And configure the precompile admin in genesis");
    }
}
