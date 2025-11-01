// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "./INativeMinter.sol";

/**
 * @title HypNativeMinter
 * @notice Hyperlane Token Router that mints native tokens using a precompile
 * @dev This contract follows Forma's implementation pattern:
 * - Mints native tokens to the contract itself
 * - Tracks locked balance
 * - Sends native tokens to recipients via sendValue
 *
 * This allows bridging TIA from Celestia as native ETH on Eden (usable for gas)
 */
contract HypNativeMinter {
    // Hyperlane Mailbox for sending/receiving messages
    address public immutable mailbox;

    // Native Minter precompile address
    INativeMinter public immutable nativeMinter;

    // Scale factor for decimal conversion (e.g., 10^12 for 6->18 decimals)
    uint256 public immutable scale;

    // Amount of native tokens locked in this contract
    uint256 private _locked;

    // Remote router addresses enrolled for each domain
    mapping(uint32 => bytes32) public routers;

    // Owner of the contract
    address public owner;

    // Reentrancy guard
    uint256 private _status;
    uint256 private constant _NOT_ENTERED = 1;
    uint256 private constant _ENTERED = 2;

    // Events
    event ReceivedMessage(uint32 origin, bytes32 sender, address recipient, uint256 amount);
    event SentMessage(uint32 destination, bytes32 recipient, address sender, uint256 amount);
    event RemoteRouterEnrolled(uint32 domain, bytes32 router);

    // Errors
    error OnlyMailbox();
    error OnlyOwner();
    error NoRouter();
    error MintFailed();
    error BurnFailed();
    error AmountExceedsLocked();
    error AmountExceedsMsgValue();
    error SendValueFailed();
    error ReentrancyGuard();

    modifier onlyMailbox() {
        if (msg.sender != mailbox) revert OnlyMailbox();
        _;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert OnlyOwner();
        _;
    }

    modifier nonReentrant() {
        if (_status == _ENTERED) revert ReentrancyGuard();
        _status = _ENTERED;
        _;
        _status = _NOT_ENTERED;
    }

    /**
     * @notice Constructor
     * @param _mailbox Hyperlane Mailbox address
     * @param _nativeMinter Native Minter precompile address
     * @param _scale Scale factor for decimal conversion
     * @param _owner Owner address
     */
    constructor(
        address _mailbox,
        address _nativeMinter,
        uint256 _scale,
        address _owner
    ) {
        mailbox = _mailbox;
        nativeMinter = INativeMinter(_nativeMinter);
        scale = _scale;
        owner = _owner;
        _status = _NOT_ENTERED;
    }

    /**
     * @notice Enroll a remote router for a specific domain
     * @param _domain The Hyperlane domain ID
     * @param _router The address of the remote router (as bytes32)
     */
    function enrollRemoteRouter(uint32 _domain, bytes32 _router) external onlyOwner {
        routers[_domain] = _router;
        emit RemoteRouterEnrolled(_domain, _router);
    }

    /**
     * @notice Transfer native tokens to a remote chain
     * @param _destination Destination domain ID
     * @param _recipient Recipient address on destination chain (as bytes32)
     * @param _amount Amount of native tokens to send (in 18 decimals)
     * @dev User sends native ETH with this call, contract burns it and sends message
     */
    function transferRemote(
        uint32 _destination,
        bytes32 _recipient,
        uint256 _amount
    ) external payable returns (bytes32 messageId) {
        // Verify msg.value covers the amount
        if (msg.value < _amount) revert AmountExceedsMsgValue();

        // Check router exists
        if (routers[_destination] == bytes32(0)) revert NoRouter();

        // Calculate gas payment (any excess from msg.value)
        uint256 gasPayment = msg.value - _amount;

        // Scale down the amount (18 decimals -> 6 decimals)
        uint256 scaledAmount = _amount / scale;
        require(scaledAmount > 0, "Destination amount < 1");

        // Verify we have enough locked tokens to burn
        if (_locked < _amount) revert AmountExceedsLocked();

        // Burn from contract's locked balance (send value with burn call)
        if (!nativeMinter.burn{value: _amount}(address(this), _amount)) revert BurnFailed();
        _locked -= _amount;

        // Encode message body
        bytes memory messageBody = abi.encodePacked(_recipient, scaledAmount);

        // Send message via Hyperlane Mailbox
        (bool success, bytes memory result) = mailbox.call{value: gasPayment}(
            abi.encodeWithSignature(
                "dispatch(uint32,bytes32,bytes)",
                _destination,
                routers[_destination],
                messageBody
            )
        );

        require(success, "Mailbox dispatch failed");
        messageId = abi.decode(result, (bytes32));

        emit SentMessage(_destination, _recipient, msg.sender, _amount);
    }

    /**
     * @notice Handle incoming Hyperlane message
     * @param _origin Origin domain ID
     * @param _sender Sender address (as bytes32)
     * @param _message Message body containing recipient and amount
     * @dev Mints to contract, then sends to recipient (Forma pattern)
     */
    function handle(
        uint32 _origin,
        bytes32 _sender,
        bytes calldata _message
    ) external onlyMailbox nonReentrant {
        // Verify sender is enrolled router
        if (routers[_origin] != _sender) revert NoRouter();

        // Decode message: recipient (32 bytes) + amount (32 bytes)
        require(_message.length >= 64, "Invalid message length");

        bytes32 recipientBytes = bytes32(_message[0:32]);
        address recipient = address(uint160(uint256(recipientBytes)));
        uint256 amount = uint256(bytes32(_message[32:64]));

        // Scale up the amount (6 decimals -> 18 decimals)
        uint256 scaledAmount = amount * scale;

        // Mint to THIS CONTRACT (Forma pattern)
        if (!nativeMinter.mint(address(this), scaledAmount)) revert MintFailed();

        // Track locked balance
        _locked += scaledAmount;

        // Send native tokens to recipient
        (bool success, ) = payable(recipient).call{value: scaledAmount}("");
        if (!success) revert SendValueFailed();

        emit ReceivedMessage(_origin, _sender, recipient, scaledAmount);
    }

    /**
     * @notice Get balance of an account (their native token balance)
     * @param _account The account to check
     * @return The native token balance
     */
    function balanceOf(address _account) external view returns (uint256) {
        return _account.balance;
    }

    /**
     * @notice Get the router address for a domain
     * @param _domain The domain ID
     * @return The router address as bytes32
     */
    function getRouter(uint32 _domain) external view returns (bytes32) {
        return routers[_domain];
    }

    /**
     * @notice Check if this contract is an admin of the native minter
     * @return True if this contract can mint tokens
     */
    function canMint() external view returns (bool) {
        return nativeMinter.isAdmin(address(this));
    }

    /**
     * @notice Get the amount of locked native tokens
     * @return Amount of native tokens locked in this contract
     */
    function locked() external view returns (uint256) {
        return _locked;
    }

    /**
     * @notice Receive function to accept native tokens
     */
    receive() external payable {}
}
