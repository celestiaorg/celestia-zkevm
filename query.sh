for i in $(seq 630 650); do
  HEX=$(printf "0x%x" $i)
  ROOT=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"$HEX\", false],\"id\":1}" \
    | jq -r '.result.stateRoot')
  echo "Block $i ($HEX): $ROOT"
done

