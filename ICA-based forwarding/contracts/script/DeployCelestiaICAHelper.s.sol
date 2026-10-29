// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.19;

import {Script, console} from "forge-std/Script.sol";
import {CelestiaICAHelper} from "../src/CelestiaICAHelper.sol";

/**
 * @title DeployCelestiaICAHelper
 * @notice Deployment script for the CelestiaICAHelper contract
 * @dev Run with: forge script script/DeployCelestiaICAHelper.s.sol --rpc-url $RPC_URL --broadcast
 */
contract DeployCelestiaICAHelper is Script {
    // Celestia ICA module address (to be set when Celestia ICA module is deployed)
    // This is a placeholder - will be updated once the Cosmos module is deployed
    bytes32 constant CELESTIA_ICA_MODULE = bytes32(uint256(0x696361000000000000000000000000000000000000000000000000000000));
    
    // Celestia ICA router address (Hyperlane ICA router identifier on Celestia)
    bytes32 constant CELESTIA_ICA_ROUTER = bytes32(uint256(0x696361726f7574657200000000000000000000000000000000000000000000));

    function run() external returns (CelestiaICAHelper helper) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        vm.startBroadcast(deployerPrivateKey);
        
        helper = new CelestiaICAHelper(CELESTIA_ICA_MODULE, CELESTIA_ICA_ROUTER);
        
        console.log("CelestiaICAHelper deployed at:", address(helper));
        console.log("FORWARD_VERSION:", vm.toString(helper.FORWARD_VERSION()));
        console.log("CELESTIA_DOMAIN:", helper.CELESTIA_DOMAIN());
        
        vm.stopBroadcast();
    }
}

