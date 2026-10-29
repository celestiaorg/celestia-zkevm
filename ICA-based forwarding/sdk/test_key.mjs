import { privateKeyToAccount } from "viem/accounts";

const relayerKey = "0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a";
const relayerAccount = privateKeyToAccount(relayerKey);
console.log("Relayer key address:", relayerAccount.address);

const anvilKey0 = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const anvilAccount0 = privateKeyToAccount(anvilKey0);
console.log("Anvil key 0 address:", anvilAccount0.address);

// Check if any maps to the owner address
const ownerAddress = "0xaF9053bB6c4346381C77C2FeD279B17ABAfCDf4d";
console.log("\nOwner address we need:", ownerAddress);

