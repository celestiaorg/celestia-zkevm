// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/HypNativeMinter.sol";
import "../src/INativeMinter.sol";

/**
 * @title HypNativeMinterTest
 * @notice Basic tests for the HypNativeMinter contract
 * @dev These are unit tests - for full integration testing, deploy to a real network with the precompile
 */
contract HypNativeMinterTest is Test {
    HypNativeMinter public hypNativeMinter;

    address constant MAILBOX = address(0x1234);
    address constant NATIVE_MINTER = address(0x0800);
    uint256 constant SCALE = 1e12;
    address constant OWNER = address(0x5678);

    function setUp() public {
        hypNativeMinter = new HypNativeMinter(
            MAILBOX,
            NATIVE_MINTER,
            SCALE,
            OWNER
        );
    }

    function test_Constructor() public view {
        assertEq(hypNativeMinter.mailbox(), MAILBOX);
        assertEq(address(hypNativeMinter.nativeMinter()), NATIVE_MINTER);
        assertEq(hypNativeMinter.scale(), SCALE);
        assertEq(hypNativeMinter.owner(), OWNER);
    }

    function test_EnrollRemoteRouter() public {
        uint32 domain = 69420; // Celestia domain
        bytes32 router = bytes32(uint256(uint160(address(0x9999))));

        vm.prank(OWNER);
        hypNativeMinter.enrollRemoteRouter(domain, router);

        assertEq(hypNativeMinter.getRouter(domain), router);
    }

    function test_EnrollRemoteRouter_OnlyOwner() public {
        uint32 domain = 69420;
        bytes32 router = bytes32(uint256(uint160(address(0x9999))));

        vm.prank(address(0xdead));
        vm.expectRevert(HypNativeMinter.OnlyOwner.selector);
        hypNativeMinter.enrollRemoteRouter(domain, router);
    }

    function test_GetRouter() public {
        uint32 domain = 69420;
        bytes32 router = bytes32(uint256(uint160(address(0x9999))));

        vm.prank(OWNER);
        hypNativeMinter.enrollRemoteRouter(domain, router);

        bytes32 storedRouter = hypNativeMinter.getRouter(domain);
        assertEq(storedRouter, router);
    }
}
