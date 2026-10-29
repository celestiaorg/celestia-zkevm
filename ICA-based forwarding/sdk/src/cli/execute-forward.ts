#!/usr/bin/env ts-node
/**
 * CLI command to execute a forwarding transaction via ICA
 */

import { Command } from "commander";
import { createPublicClient, createWalletClient, http, parseEther } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { prepareForwarding, addressToBytes32 } from "../index";
import { CHAINS } from "../constants";

// InterchainAccountRouter ABI (relevant functions only)
const ICA_ROUTER_ABI = [
  {
    name: "callRemoteWithOverrides",
    type: "function",
    inputs: [
      { name: "_destination", type: "uint32" },
      { name: "_router", type: "bytes32" },
      { name: "_ism", type: "bytes32" },
      { name: "_calls", type: "tuple[]", components: [
        { name: "to", type: "bytes32" },
        { name: "value", type: "uint256" },
        { name: "data", type: "bytes" },
      ]},
      { name: "_hookMetadata", type: "bytes" },
      { name: "_userSalt", type: "bytes32" },
    ],
    outputs: [{ type: "bytes32" }],
    stateMutability: "payable",
  },
  {
    name: "quoteGasPayment",
    type: "function",
    inputs: [
      { name: "_destination", type: "uint32" },
      { name: "_gasLimit", type: "uint256" },
    ],
    outputs: [{ type: "uint256" }],
    stateMutability: "view",
  },
] as const;

const program = new Command();

program
  .name("execute-forward")
  .description("Execute a forwarding transaction via ICA")
  .requiredOption("--token <tokenId>", "Warp token ID on Celestia (bytes32 hex)")
  .requiredOption("--dest-domain <domain>", "Destination domain ID", parseInt)
  .requiredOption("--recipient <address>", "Final recipient address")
  .requiredOption("--amount <amount>", "Amount to forward (in wei)", BigInt)
  .option("--private-key <key>", "Private key for signing", process.env.PRIVATE_KEY)
  .option("--rpc-url <url>", "RPC URL", CHAINS.rethlocal.rpcUrl)
  .option("--dry-run", "Simulate without executing")
  .action(async (options) => {
    if (!options.privateKey) {
      console.error("Error: Private key required (--private-key or PRIVATE_KEY env)");
      process.exit(1);
    }

    // Normalize recipient to bytes32
    const recipientBytes32 = options.recipient.length === 42
      ? addressToBytes32(options.recipient)
      : options.recipient;

    console.log("\n=== Preparing Forwarding Transaction ===\n");

    // Prepare forwarding data
    const { forwardingAddress, icaCall, icaMessage } = prepareForwarding(
      options.token,
      options.destDomain,
      recipientBytes32,
      BigInt(options.amount)
    );

    console.log("Forwarding Intent:");
    console.log(`  Token ID:         ${options.token}`);
    console.log(`  Dest Domain:      ${options.destDomain}`);
    console.log(`  Recipient:        ${recipientBytes32}`);
    console.log(`  Amount:           ${options.amount}`);
    console.log(`\nForwarding Address: ${forwardingAddress.celestiaAddress}`);
    console.log(`Salt:               ${forwardingAddress.salt}`);

    // Set up clients
    const account = privateKeyToAccount(options.privateKey as `0x${string}`);
    const publicClient = createPublicClient({
      transport: http(options.rpcUrl),
    });
    const walletClient = createWalletClient({
      account,
      transport: http(options.rpcUrl),
    });

    // Quote gas payment
    const gasQuote = await publicClient.readContract({
      address: CHAINS.rethlocal.icaRouter as `0x${string}`,
      abi: ICA_ROUTER_ABI,
      functionName: "quoteGasPayment",
      args: [CHAINS.celestia.domainId, 500000n],
    });

    console.log(`\nGas Quote: ${gasQuote} wei`);

    if (options.dryRun) {
      console.log("\n[DRY RUN] Would execute ICA call with:");
      console.log(`  Destination: ${CHAINS.celestia.domainId}`);
      console.log(`  Router:      ${CHAINS.celestia.icaRouter}`);
      console.log(`  ISM:         ${CHAINS.celestia.ism}`);
      console.log(`  Call To:     ${icaCall.to}`);
      console.log(`  Call Value:  ${icaCall.value}`);
      console.log(`  User Salt:   ${forwardingAddress.salt}`);
      return;
    }

    console.log("\nExecuting ICA call...");

    // Execute the ICA call
    const hash = await walletClient.writeContract({
      address: CHAINS.rethlocal.icaRouter as `0x${string}`,
      abi: ICA_ROUTER_ABI,
      functionName: "callRemoteWithOverrides",
      args: [
        CHAINS.celestia.domainId,
        CHAINS.celestia.icaRouter as `0x${string}`,
        CHAINS.celestia.ism as `0x${string}`,
        [{
          to: icaCall.to as `0x${string}`,
          value: icaCall.value,
          data: icaCall.data as `0x${string}`,
        }],
        "0x" as `0x${string}`, // empty hook metadata
        forwardingAddress.salt as `0x${string}`,
      ],
      value: gasQuote,
    });

    console.log(`\nTransaction submitted: ${hash}`);

    // Wait for confirmation
    const receipt = await publicClient.waitForTransactionReceipt({ hash });
    console.log(`Transaction confirmed in block ${receipt.blockNumber}`);
    console.log(`Status: ${receipt.status === "success" ? "✅ Success" : "❌ Failed"}`);
  });

program.parse();

