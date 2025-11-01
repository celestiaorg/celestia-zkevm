// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/**
 * @title MockMailbox
 * @notice Mock implementation of Hyperlane Mailbox for testing
 * @dev This allows you to test the full HypNativeMinter flow without a real Hyperlane deployment
 */
contract MockMailbox {
    uint256 public nonce;

    // Events matching Hyperlane's Mailbox
    event Dispatch(
        address indexed sender,
        uint32 indexed destination,
        bytes32 indexed recipient,
        bytes message
    );

    event Process(
        uint32 indexed origin,
        bytes32 indexed sender,
        address indexed recipient
    );

    /**
     * @notice Send a message via Hyperlane (mocked)
     * @param destination Destination domain ID
     * @param recipient Recipient address (as bytes32)
     * @param messageBody Message body
     * @return messageId The ID of the message
     */
    function dispatch(
        uint32 destination,
        bytes32 recipient,
        bytes calldata messageBody
    ) external returns (bytes32 messageId) {
        messageId = keccak256(abi.encodePacked(nonce++, msg.sender, destination, recipient, messageBody));

        emit Dispatch(msg.sender, destination, recipient, messageBody);

        return messageId;
    }

    /**
     * @notice Process an incoming message (manual trigger for testing)
     * @param origin Origin domain ID
     * @param sender Sender address (as bytes32)
     * @param recipient Recipient contract address
     * @param messageBody Message body
     */
    function processMessage(
        uint32 origin,
        bytes32 sender,
        address recipient,
        bytes calldata messageBody
    ) external {
        emit Process(origin, sender, recipient);

        // Call the recipient's handle function
        (bool success,) = recipient.call(
            abi.encodeWithSignature(
                "handle(uint32,bytes32,bytes)",
                origin,
                sender,
                messageBody
            )
        );

        require(success, "Message processing failed");
    }
}
