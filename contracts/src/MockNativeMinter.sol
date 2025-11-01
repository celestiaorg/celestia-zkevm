// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "./INativeMinter.sol";

/**
 * @title MockNativeMinter
 * @notice Mock implementation of the Native Minter precompile for testing
 * @dev This contract simulates the behavior of the native minter precompile
 * allowing you to test the full Hyperlane flow locally without needing ev-reth
 *
 * Deploy this at any address and use it as the nativeMinter parameter
 * when deploying HypNativeMinter for testing.
 */
contract MockNativeMinter is INativeMinter {
    // Mapping of admin addresses
    mapping(address => bool) public admins;

    // Events
    event Minted(address indexed recipient, uint256 amount);
    event Burned(address indexed burner, uint256 amount);
    event AdminAdded(address indexed admin);
    event AdminRemoved(address indexed admin);

    constructor() {
        // Deployer is the initial admin
        admins[msg.sender] = true;
        emit AdminAdded(msg.sender);
    }

    /**
     * @notice Add an admin (for testing purposes)
     * @param admin Address to grant admin privileges
     */
    function addAdmin(address admin) external {
        require(admins[msg.sender], "Only admin");
        admins[admin] = true;
        emit AdminAdded(admin);
    }

    /**
     * @notice Remove an admin (for testing purposes)
     * @param admin Address to revoke admin privileges
     */
    function removeAdmin(address admin) external {
        require(admins[msg.sender], "Only admin");
        admins[admin] = false;
        emit AdminRemoved(admin);
    }

    /**
     * @notice Mints native tokens to a recipient address
     * @param recipient The address that will receive the minted native tokens
     * @param amount The amount of native tokens to mint (in wei)
     * @return success True if the mint was successful
     */
    function mint(address recipient, uint256 amount) external override returns (bool success) {
        require(admins[msg.sender], "Not admin");
        require(recipient != address(0), "Invalid recipient");
        require(amount > 0, "Invalid amount");

        // Transfer native tokens to recipient
        // In the real precompile, this would actually mint new tokens
        // For the mock, we'll use the contract's balance
        (bool sent,) = recipient.call{value: amount}("");
        require(sent, "Transfer failed");

        emit Minted(recipient, amount);
        return true;
    }

    /**
     * @notice Burns native tokens from a specific address
     * @param from The address to burn tokens from
     * @param amount The amount of native tokens to burn (in wei)
     * @return success True if the burn was successful
     * @dev In the mock, we accept tokens sent to this contract (simulating burn)
     */
    function burn(address from, uint256 amount) external payable override returns (bool success) {
        require(msg.value == amount, "Value mismatch");
        require(amount > 0, "Invalid amount");
        require(from != address(0), "Invalid from address");

        // In the real precompile, this would actually burn tokens
        // For the mock, we just accept the tokens (they stay in this contract)
        // This simulates the burn by removing them from circulation

        emit Burned(from, amount);
        return true;
    }

    /**
     * @notice Checks if an address has admin privileges
     * @param account The address to check
     * @return isAdmin True if the address is an admin
     */
    function isAdmin(address account) external view override returns (bool) {
        return admins[account];
    }

    /**
     * @notice Fund the mock with native tokens (for minting)
     * @dev In production, the precompile can mint from nothing
     * For the mock, we need to fund it so it can "mint" (transfer) to recipients
     */
    receive() external payable {}

    /**
     * @notice Get the contract's balance (available for minting)
     */
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}
