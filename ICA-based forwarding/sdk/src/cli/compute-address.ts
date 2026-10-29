#!/usr/bin/env ts-node
/**
 * CLI command to compute a forwarding address
 */

import { Command } from "commander";
import { computeForwardingAddress, addressToBytes32 } from "../address";
import { ForwardingIntent } from "../types";
import { CHAINS } from "../constants";

const program = new Command();

program
  .name("compute-address")
  .description("Compute a Celestia ICA forwarding address")
  .requiredOption("--token <tokenId>", "Warp token ID on Celestia (bytes32 hex)")
  .requiredOption("--dest-domain <domain>", "Destination domain ID", parseInt)
  .requiredOption("--recipient <address>", "Final recipient address (EVM or bytes32)")
  .option("--json", "Output as JSON")
  .action((options) => {
    // Normalize recipient to bytes32
    const recipientBytes32 = options.recipient.length === 42
      ? addressToBytes32(options.recipient)
      : options.recipient;

    const intent: ForwardingIntent = {
      tokenId: options.token,
      destDomain: options.destDomain,
      destRecipient: recipientBytes32,
    };

    const result = computeForwardingAddress(intent);

    if (options.json) {
      console.log(JSON.stringify({
        intent: {
          tokenId: intent.tokenId,
          destDomain: intent.destDomain,
          destRecipient: intent.destRecipient,
        },
        result: {
          celestiaAddress: result.celestiaAddress,
          addressBytes32: result.addressBytes32,
          salt: result.salt,
          callDigest: result.callDigest,
        },
      }, null, 2));
    } else {
      console.log("\n=== Forwarding Address Computation ===\n");
      console.log("Intent:");
      console.log(`  Token ID:      ${intent.tokenId}`);
      console.log(`  Dest Domain:   ${intent.destDomain}`);
      console.log(`  Recipient:     ${intent.destRecipient}`);
      console.log("\nResult:");
      console.log(`  Celestia Addr: ${result.celestiaAddress}`);
      console.log(`  Bytes32:       ${result.addressBytes32}`);
      console.log(`  Salt:          ${result.salt}`);
      console.log(`  Call Digest:   ${result.callDigest}`);
      console.log("\nUsage:");
      console.log(`  1. Send tokens to ${result.celestiaAddress} via warp route`);
      console.log(`  2. Trigger forwarding via ICA call with salt: ${result.salt}`);
      console.log("");
    }
  });

program.parse();

