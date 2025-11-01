// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/HypNativeMinter.sol";
import "../src/MockNativeMinter.sol";
import "../src/MockMailbox.sol";

/**
 * @title HypNativeMinterIntegrationTest
 * @notice Full integration test for HypNativeMinter with mocked components
 * @dev This test simulates the complete flow: Celestia -> Eden -> back
 */
contract HypNativeMinterIntegrationTest is Test {
    MockNativeMinter public nativeMinter;
    MockMailbox public mailbox;
    HypNativeMinter public hypNativeMinter;

    address public owner = address(0x1);
    address public alice = address(0x2); // User on Eden
    address public bob = address(0x3);   // User on Celestia

    uint32 constant CELESTIA_DOMAIN = 69420;
    uint32 constant EDEN_DOMAIN = 1234;
    uint256 constant SCALE = 1e12; // 6 decimals to 18 decimals

    // Celestia router (mocked as a simple bytes32)
    bytes32 celestiaRouter;

    function setUp() public {
        // Deploy mock contracts
        mailbox = new MockMailbox();
        nativeMinter = new MockNativeMinter();

        // Deploy HypNativeMinter
        vm.prank(owner);
        hypNativeMinter = new HypNativeMinter(
            address(mailbox),
            address(nativeMinter),
            SCALE,
            owner
        );

        // Set HypNativeMinter as admin of the native minter
        nativeMinter.addAdmin(address(hypNativeMinter));

        // Fund the mock native minter (so it can "mint" tokens)
        vm.deal(address(nativeMinter), 1000 ether);

        // Set up the Celestia router
        celestiaRouter = bytes32(uint256(uint160(address(0x9999))));

        // Enroll the Celestia router
        vm.prank(owner);
        hypNativeMinter.enrollRemoteRouter(CELESTIA_DOMAIN, celestiaRouter);

        // Give alice some initial balance for gas
        vm.deal(alice, 10 ether);
    }

    function test_FullFlow_CelestiaToEdenAndBack() public {
        console.log("\n=== Testing Full Bridge Flow ===\n");

        // 1. Simulate bridging FROM Celestia TO Eden
        console.log("1. Bridging TIA from Celestia to Eden...");

        uint256 amountOnCelestia = 1_000_000; // 1 TIA (6 decimals)
        uint256 expectedAmountOnEden = amountOnCelestia * SCALE; // Scaled to 18 decimals

        // Build the message body: recipient (32 bytes) + amount (32 bytes)
        bytes32 aliceBytes32 = bytes32(uint256(uint160(alice)));
        bytes memory messageBody = abi.encodePacked(aliceBytes32, amountOnCelestia);

        uint256 aliceBalanceBefore = alice.balance;

        // Process incoming message from Celestia (as if Hyperlane relayer delivered it)
        mailbox.processMessage(
            CELESTIA_DOMAIN,
            celestiaRouter,
            address(hypNativeMinter),
            messageBody
        );

        uint256 aliceBalanceAfter = alice.balance;

        console.log("Alice balance before:", aliceBalanceBefore);
        console.log("Alice balance after:", aliceBalanceAfter);
        console.log("Expected amount:", expectedAmountOnEden);

        assertEq(aliceBalanceAfter - aliceBalanceBefore, expectedAmountOnEden, "Alice should receive scaled TIA");

        // 2. Simulate bridging FROM Eden BACK to Celestia
        console.log("\n2. Bridging TIA from Eden back to Celestia...");

        uint256 amountToBridgeBack = 500_000 * SCALE; // 0.5 TIA in 18 decimals
        bytes32 bobBytes32 = bytes32(uint256(uint160(bob)));

        uint256 aliceBalanceBeforeBurn = alice.balance;

        // Alice bridges back to Celestia
        vm.prank(alice);
        hypNativeMinter.transferRemote{value: amountToBridgeBack}(
            CELESTIA_DOMAIN,
            bobBytes32,
            amountToBridgeBack
        );

        uint256 aliceBalanceAfterBurn = alice.balance;

        console.log("Alice balance before burn:", aliceBalanceBeforeBurn);
        console.log("Alice balance after burn:", aliceBalanceAfterBurn);

        // Alice should have burned the tokens
        assertEq(
            aliceBalanceBeforeBurn - aliceBalanceAfterBurn,
            amountToBridgeBack,
            "Alice should burn native TIA"
        );

        console.log("\n=== Full Flow Test Complete ===");
    }

    function test_CanMint() public view {
        bool canMint = hypNativeMinter.canMint();
        assertTrue(canMint, "HypNativeMinter should be able to mint");
    }

    function test_EnrollRemoteRouter() public {
        bytes32 newRouter = bytes32(uint256(uint160(address(0x8888))));
        uint32 newDomain = 12345;

        vm.prank(owner);
        hypNativeMinter.enrollRemoteRouter(newDomain, newRouter);

        bytes32 storedRouter = hypNativeMinter.getRouter(newDomain);
        assertEq(storedRouter, newRouter, "Router should be enrolled");
    }

    function testFail_MintWithoutAdmin() public {
        // Deploy a new HypNativeMinter without admin privileges
        HypNativeMinter unauthorizedMinter = new HypNativeMinter(
            address(mailbox),
            address(nativeMinter),
            SCALE,
            owner
        );

        // Try to process a message (should fail because it's not an admin)
        bytes32 aliceBytes32 = bytes32(uint256(uint160(alice)));
        bytes memory messageBody = abi.encodePacked(aliceBytes32, uint256(1_000_000));

        vm.prank(owner);
        unauthorizedMinter.enrollRemoteRouter(CELESTIA_DOMAIN, celestiaRouter);

        // This should fail because unauthorizedMinter is not an admin of nativeMinter
        mailbox.processMessage(
            CELESTIA_DOMAIN,
            celestiaRouter,
            address(unauthorizedMinter),
            messageBody
        );
    }

    function test_RevertOnUnauthorizedRouter() public {
        bytes32 unauthorizedRouter = bytes32(uint256(uint160(address(0x7777))));
        bytes32 aliceBytes32 = bytes32(uint256(uint160(alice)));
        bytes memory messageBody = abi.encodePacked(aliceBytes32, uint256(1_000_000));

        // Should revert because the router is not enrolled
        // The MockMailbox wraps the revert, so we check for "Message processing failed"
        vm.expectRevert("Message processing failed");
        mailbox.processMessage(
            CELESTIA_DOMAIN,
            unauthorizedRouter,
            address(hypNativeMinter),
            messageBody
        );
    }

    function test_DecimalScaling() public {
        // Test that 1 TIA (6 decimals) = 1 TIA (18 decimals)
        uint256 oneTiaCelestia = 1_000_000; // 6 decimals
        uint256 oneTiaEden = oneTiaCelestia * SCALE; // 18 decimals

        assertEq(oneTiaEden, 1e18, "1 TIA should equal 1e18 in 18 decimals");

        // Simulate minting
        bytes32 aliceBytes32 = bytes32(uint256(uint160(alice)));
        bytes memory messageBody = abi.encodePacked(aliceBytes32, oneTiaCelestia);

        uint256 balanceBefore = alice.balance;

        mailbox.processMessage(
            CELESTIA_DOMAIN,
            celestiaRouter,
            address(hypNativeMinter),
            messageBody
        );

        uint256 balanceAfter = alice.balance;

        assertEq(balanceAfter - balanceBefore, 1e18, "Should mint exactly 1 TIA in 18 decimals");
    }

    function test_MultipleBridges() public {
        // Test multiple sequential bridges
        for (uint256 i = 1; i <= 5; i++) {
            uint256 amount = i * 100_000; // Varying amounts
            bytes32 aliceBytes32 = bytes32(uint256(uint160(alice)));
            bytes memory messageBody = abi.encodePacked(aliceBytes32, amount);

            uint256 balanceBefore = alice.balance;

            mailbox.processMessage(
                CELESTIA_DOMAIN,
                celestiaRouter,
                address(hypNativeMinter),
                messageBody
            );

            uint256 balanceAfter = alice.balance;
            uint256 expected = amount * SCALE;

            assertEq(
                balanceAfter - balanceBefore,
                expected,
                string(abi.encodePacked("Bridge #", vm.toString(i), " failed"))
            );
        }
    }
}
