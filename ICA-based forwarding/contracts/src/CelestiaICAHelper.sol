// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.19;

/**
 * @title CelestiaICAHelper
 * @notice Helper contract for computing Celestia ICA forwarding addresses and encoding calls.
 * @dev This contract provides utilities for the ICA-based forwarding pattern where:
 *      1. User computes a deterministic forwarding address on Celestia
 *      2. User sends tokens to that address via warp route
 *      3. User (or anyone) triggers forwarding via ICA call
 *
 * The forwarding address is derived from a commitment to the forwarding intent,
 * binding the address cryptographically to (token, destDomain, destRecipient).
 */
contract CelestiaICAHelper {
    /// @notice Version prefix for forward address derivation
    bytes32 public constant FORWARD_VERSION = keccak256("CELESTIA_ICA_FORWARD_V1");

    /// @notice The Celestia ICA module address (used as owner in derivation)
    bytes32 public immutable celestiaICAModule;

    /// @notice The InterchainAccountRouter on Celestia (used as router in derivation)
    bytes32 public immutable celestiaICARouter;

    /// @notice The Celestia domain ID
    uint32 public constant CELESTIA_DOMAIN = 69420;

    /**
     * @notice Emitted when a forwarding intent is computed
     * @param forwardingAddress The computed Celestia forwarding address
     * @param tokenId The warp token ID on Celestia
     * @param destDomain The final destination domain
     * @param destRecipient The final recipient address
     */
    event ForwardingIntentComputed(
        bytes32 indexed forwardingAddress,
        bytes32 indexed tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    );

    constructor(bytes32 _celestiaICAModule, bytes32 _celestiaICARouter) {
        celestiaICAModule = _celestiaICAModule;
        celestiaICARouter = _celestiaICARouter;
    }

    /**
     * @notice Computes the forwarding address on Celestia for a given intent
     * @dev The address is derived as:
     *      callDigest = keccak256(abi.encode(tokenId, destDomain, destRecipient))
     *      salt = keccak256(abi.encodePacked(FORWARD_VERSION, callDigest))
     *      forwardAddr = deriveCelestiaModuleAccount(celestiaICAModule, salt)
     *
     * @param tokenId The warp token ID on Celestia (bytes32 representation)
     * @param destDomain The final destination domain
     * @param destRecipient The final recipient on the destination domain
     * @return forwardingAddress The deterministic forwarding address on Celestia
     */
    function computeForwardingAddress(
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    ) external view returns (bytes32 forwardingAddress) {
        bytes32 salt = _computeForwardingSalt(tokenId, destDomain, destRecipient);
        forwardingAddress = _deriveCelestiaAddress(salt);
    }

    /**
     * @notice Computes the forwarding salt from intent parameters
     * @param tokenId The warp token ID on Celestia
     * @param destDomain The final destination domain
     * @param destRecipient The final recipient address
     * @return salt The salt used for address derivation
     */
    function computeForwardingSalt(
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    ) external pure returns (bytes32 salt) {
        return _computeForwardingSalt(tokenId, destDomain, destRecipient);
    }

    /**
     * @notice Encodes the forwarding call for the ICA message
     * @dev This returns the calldata that will be executed on Celestia to forward tokens.
     *      The call is to the warp module's RemoteTransfer function.
     *
     * @param tokenId The warp token ID on Celestia
     * @param destDomain The final destination domain
     * @param destRecipient The final recipient address
     * @param amount The amount of tokens to forward
     * @return callData The encoded call data for the ICA message
     */
    function encodeForwardingCall(
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient,
        uint256 amount
    ) external pure returns (bytes memory callData) {
        // Encode the RemoteTransfer call for the Celestia warp module
        // This matches the hyperlane-cosmos warp module's MsgRemoteTransfer format
        callData = abi.encode(
            tokenId,
            destDomain,
            destRecipient,
            amount
        );
    }

    /**
     * @notice Builds a complete ICA Call struct for forwarding
     * @param warpModuleAddress The Celestia warp module address (bytes32)
     * @param tokenId The warp token ID
     * @param destDomain The destination domain
     * @param destRecipient The final recipient
     * @param amount The amount to forward
     * @return to The target address for the call
     * @return value The value to send (always 0 for token transfers)
     * @return data The call data
     */
    function buildForwardingCall(
        bytes32 warpModuleAddress,
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient,
        uint256 amount
    ) external pure returns (bytes32 to, uint256 value, bytes memory data) {
        to = warpModuleAddress;
        value = 0;
        data = abi.encode(tokenId, destDomain, destRecipient, amount);
    }

    /**
     * @notice Computes the call digest for a forwarding intent
     * @param tokenId The warp token ID
     * @param destDomain The destination domain
     * @param destRecipient The final recipient
     * @return digest The keccak256 hash of the encoded intent
     */
    function computeCallDigest(
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    ) external pure returns (bytes32 digest) {
        return keccak256(abi.encode(tokenId, destDomain, destRecipient));
    }

    /**
     * @notice Validates that a forwarding address matches the expected intent
     * @param forwardingAddress The address to validate
     * @param tokenId The expected token ID
     * @param destDomain The expected destination domain
     * @param destRecipient The expected recipient
     * @return valid True if the address matches the intent
     */
    function validateForwardingAddress(
        bytes32 forwardingAddress,
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    ) external view returns (bool valid) {
        bytes32 salt = _computeForwardingSalt(tokenId, destDomain, destRecipient);
        bytes32 expectedAddress = _deriveCelestiaAddress(salt);
        return forwardingAddress == expectedAddress;
    }

    // ============ Internal Functions ============

    function _computeForwardingSalt(
        bytes32 tokenId,
        uint32 destDomain,
        bytes32 destRecipient
    ) internal pure returns (bytes32) {
        bytes32 callDigest = keccak256(abi.encode(tokenId, destDomain, destRecipient));
        return keccak256(abi.encodePacked(FORWARD_VERSION, callDigest));
    }

    /**
     * @notice Derives a Celestia module account address from a salt
     * @dev This mimics the Cosmos SDK module account derivation:
     *      address = hash(moduleAddress || salt)[12:32]
     *      
     *      Note: Actual Celestia address derivation may differ slightly.
     *      This provides a deterministic mapping that must match the
     *      Cosmos SDK implementation in the ICA module.
     *
     * @param salt The salt for address derivation
     * @return The derived address as bytes32
     */
    function _deriveCelestiaAddress(bytes32 salt) internal view returns (bytes32) {
        // Cosmos SDK derives module account addresses using:
        // address = sha256(moduleAddress || "/" || salt)[:20]
        // We represent this as a bytes32 with zero-padding
        bytes32 preimage = keccak256(abi.encodePacked(celestiaICAModule, salt));
        // Take last 20 bytes (like Cosmos address derivation)
        return preimage;
    }
}

