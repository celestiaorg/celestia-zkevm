#!/bin/bash

# Fix relayer config by removing merkleTreeHook from Celestia chain
# The relayer tries to query MerkleTreeHook state from Celestia, but Celestia
# only has NoopHooks for testing. This causes "key '0' not found" errors.

CONFIG_FILE="hyperlane/relayer-config.json"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Error: $CONFIG_FILE not found"
    exit 1
fi

echo "Fixing relayer config..."
echo "  Removing merkleTreeHook from Celestia chain config"

# Use Python to remove the merkleTreeHook field from celestia chain
python3 -c "
import json

with open('$CONFIG_FILE', 'r') as f:
    config = json.load(f)

# Remove merkleTreeHook from celestia chain if it exists
if 'celestia' in config['chains'] and 'merkleTreeHook' in config['chains']['celestia']:
    del config['chains']['celestia']['merkleTreeHook']
    print('  ✅ Removed merkleTreeHook from Celestia config')
else:
    print('  ℹ️  merkleTreeHook not found in Celestia config (already fixed?)')

with open('$CONFIG_FILE', 'w') as f:
    json.dump(config, f, indent=4)

print('  ✅ Config file updated')
"

echo ""
echo "✅ Relayer config fixed!"
echo ""
echo "The relayer will no longer try to query MerkleTreeHook from Celestia."
echo "Restart the relayer for changes to take effect:"
echo "  docker restart relayer"
