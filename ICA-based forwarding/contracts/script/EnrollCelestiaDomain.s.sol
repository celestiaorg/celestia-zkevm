// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.19;

import {Script, console} from "forge-std/Script.sol";

/**
 * @title EnrollCelestiaDomain
 * @notice Script to enroll Celestia domain (69420) on the InterchainAccountRouter
 * @dev This configures the ICA router to recognize the Celestia ICA module as a remote router
 *
 * Run with: 
 *   forge script script/EnrollCelestiaDomain.s.sol --rpc-url http://localhost:8545 --broadcast
 */
contract EnrollCelestiaDomain is Script {
    // Deployed InterchainAccountRouter on rethlocal (EVM Chain 1)
    address constant ICA_ROUTER = 0x4dc4E8bf5D0390C95Af9AFEb1e9c9927c4dB83e7;
    
    // Celestia domain ID
    uint32 constant CELESTIA_DOMAIN = 69420;
    
    // Celestia ICA module address (bytes32 representation)
    // This will be the hyperlane-cosmos ICA module's identifier
    // Format: module_name padded to bytes32
    bytes32 constant CELESTIA_ICA_ROUTER = 0x6963615f726f7574657200000000000000000000000000000000000000000000;
    
    // Celestia ISM address (using NoOp ISM for prototyping)
    // This should match the ISM ID configured on Celestia
    bytes32 constant CELESTIA_ISM = 0x726f757465725f69736d00000000000000000000000000000000000000000000;

    // InterchainAccountRouter interface for enrollment
    interface IInterchainAccountRouter {
        function enrollRemoteRouterAndIsm(
            uint32 _destination,
            bytes32 _router,
            bytes32 _ism
        ) external;
        
        function routers(uint32 _domain) external view returns (bytes32);
        function isms(uint32 _domain) external view returns (bytes32);
        function owner() external view returns (address);
    }

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        IInterchainAccountRouter router = IInterchainAccountRouter(ICA_ROUTER);
        
        console.log("InterchainAccountRouter:", ICA_ROUTER);
        console.log("Router owner:", router.owner());
        console.log("Celestia domain:", CELESTIA_DOMAIN);
        
        // Check if already enrolled
        bytes32 existingRouter = router.routers(CELESTIA_DOMAIN);
        if (existingRouter != bytes32(0)) {
            console.log("Celestia domain already enrolled!");
            console.log("Existing router:", vm.toString(existingRouter));
            console.log("Existing ISM:", vm.toString(router.isms(CELESTIA_DOMAIN)));
            return;
        }
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Enroll Celestia domain with its ICA router and ISM
        router.enrollRemoteRouterAndIsm(
            CELESTIA_DOMAIN,
            CELESTIA_ICA_ROUTER,
            CELESTIA_ISM
        );
        
        vm.stopBroadcast();
        
        // Verify enrollment
        bytes32 enrolledRouter = router.routers(CELESTIA_DOMAIN);
        bytes32 enrolledIsm = router.isms(CELESTIA_DOMAIN);
        
        console.log("Successfully enrolled Celestia domain!");
        console.log("Enrolled router:", vm.toString(enrolledRouter));
        console.log("Enrolled ISM:", vm.toString(enrolledIsm));
    }
}

