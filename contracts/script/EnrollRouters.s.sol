// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/HypNativeMinter.sol";

/**
 * @title EnrollRouters
 * @notice Helper script to enroll remote routers on HypNativeMinter
 *
 * Usage:
 *   forge script script/EnrollRouters.s.sol:EnrollRouters \
 *     --rpc-url http://localhost:8545 \
 *     --private-key 0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a \
 *     --broadcast \
 *     --sig "run(address,uint32,bytes32)" \
 *     <HYPNATIVEMINTER_ADDRESS> \
 *     69420 \
 *     <CELESTIA_ROUTER_AS_BYTES32>
 */
contract EnrollRouters is Script {
    function run(
        address hypNativeMinterAddress,
        uint32 remoteDomain,
        bytes32 remoteRouter
    ) external {
        console.log("=== Enrolling Remote Router ===");
        console.log("HypNativeMinter:", hypNativeMinterAddress);
        console.log("Remote Domain:", remoteDomain);
        console.log("Remote Router:", vm.toString(remoteRouter));

        vm.startBroadcast();

        HypNativeMinter hypNativeMinter = HypNativeMinter(payable(hypNativeMinterAddress));
        hypNativeMinter.enrollRemoteRouter(remoteDomain, remoteRouter);

        vm.stopBroadcast();

        console.log("\nRouter enrolled successfully!");

        // Verify
        bytes32 enrolled = hypNativeMinter.getRouter(remoteDomain);
        console.log("Verification - Stored router:", vm.toString(enrolled));
        require(enrolled == remoteRouter, "Router mismatch!");
    }

    /**
     * @notice Helper function to convert address to bytes32
     * Usage in cast:
     *   cast call <SCRIPT_ADDRESS> "addressToBytes32(address)(bytes32)" <ADDRESS>
     */
    function addressToBytes32(address addr) public pure returns (bytes32) {
        return bytes32(uint256(uint160(addr)));
    }
}
