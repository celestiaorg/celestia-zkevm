#!/usr/bin/env node
/**
 * Main CLI entry point for ICA forwarding tools
 */

import { Command } from "commander";

const program = new Command();

program
  .name("ica-forward")
  .description("ICA-based forwarding tools for Celestia")
  .version("0.1.0");

program
  .command("compute-address")
  .description("Compute a forwarding address on Celestia")
  .requiredOption("--token <tokenId>", "Warp token ID on Celestia (bytes32 hex)")
  .requiredOption("--dest-domain <domain>", "Destination domain ID")
  .requiredOption("--recipient <address>", "Final recipient address")
  .option("--json", "Output as JSON")
  .action(async (options) => {
    // Dynamically import to avoid loading heavy deps until needed
    const { computeForwardingAddress, addressToBytes32 } = await import("../address");
    const { ForwardingIntent } = await import("../types");

    const recipientBytes32 = options.recipient.length === 42
      ? addressToBytes32(options.recipient)
      : options.recipient;

    const intent = {
      tokenId: options.token,
      destDomain: parseInt(options.destDomain),
      destRecipient: recipientBytes32,
    };

    const result = computeForwardingAddress(intent);

    if (options.json) {
      console.log(JSON.stringify({ intent, result }, null, 2));
    } else {
      console.log("\n=== Forwarding Address ===");
      console.log(`Celestia: ${result.celestiaAddress}`);
      console.log(`Bytes32:  ${result.addressBytes32}`);
      console.log(`Salt:     ${result.salt}`);
    }
  });

program
  .command("validate-address")
  .description("Validate a forwarding address matches an intent")
  .requiredOption("--address <address>", "Address to validate")
  .requiredOption("--token <tokenId>", "Expected token ID")
  .requiredOption("--dest-domain <domain>", "Expected destination domain")
  .requiredOption("--recipient <address>", "Expected recipient")
  .action(async (options) => {
    const { validateForwardingAddress, addressToBytes32 } = await import("../address");

    const recipientBytes32 = options.recipient.length === 42
      ? addressToBytes32(options.recipient)
      : options.recipient;

    const intent = {
      tokenId: options.token,
      destDomain: parseInt(options.destDomain),
      destRecipient: recipientBytes32,
    };

    const valid = validateForwardingAddress(options.address, intent);
    
    if (valid) {
      console.log("✅ Address is valid for the given intent");
    } else {
      console.log("❌ Address does NOT match the intent");
      process.exit(1);
    }
  });

program
  .command("encode-call")
  .description("Encode a forwarding call for ICA execution")
  .requiredOption("--token <tokenId>", "Warp token ID")
  .requiredOption("--dest-domain <domain>", "Destination domain")
  .requiredOption("--recipient <address>", "Recipient address")
  .requiredOption("--amount <amount>", "Amount to forward (in base units)")
  .action(async (options) => {
    const { encodeForwardingCallData, addressToBytes32 } = await import("../encoding");
    const addressUtils = await import("../address");

    const recipientBytes32 = options.recipient.length === 42
      ? addressUtils.addressToBytes32(options.recipient)
      : options.recipient;

    const intent = {
      tokenId: options.token,
      destDomain: parseInt(options.destDomain),
      destRecipient: recipientBytes32,
    };

    const callData = encodeForwardingCallData(intent, BigInt(options.amount));
    
    console.log("\n=== Encoded Call Data ===");
    console.log(callData);
  });

program.parse();

