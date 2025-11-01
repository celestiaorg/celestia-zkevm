// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/**
 * @title INativeMinter
 * @notice Interface for the Native Minter precompile
 * @dev This precompile allows whitelisted addresses to mint the native token (e.g., TIA on Eden)
 *
 * The precompile is typically deployed at a fixed address (e.g., 0x0000000000000000000000000000000000000800)
 * Only addresses with admin privileges can mint native tokens
 */
interface INativeMinter {
    /**
     * @notice Mints native tokens to a recipient address
     * @param recipient The address that will receive the minted native tokens
     * @param amount The amount of native tokens to mint (in wei)
     * @return success True if the mint was successful
     */
    function mint(address recipient, uint256 amount) external returns (bool success);

    /**
     * @notice Burns native tokens from a specific address
     * @param from The address to burn tokens from
     * @param amount The amount of native tokens to burn (in wei)
     * @return success True if the burn was successful
     */
    function burn(address from, uint256 amount) external payable returns (bool success);

    /**
     * @notice Checks if an address has admin privileges
     * @param account The address to check
     * @return isAdmin True if the address is an admin
     */
    function isAdmin(address account) external view returns (bool isAdmin);
}
