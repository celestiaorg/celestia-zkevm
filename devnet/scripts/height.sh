curl -s http://localhost:26657/status | jq -r '.result.sync_info.latest_block_height'
