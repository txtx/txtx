1. Right now, we start and stop the anvil process. However, anvil has snapshot, and revert capabilities.

snapshot/revert explained
- you can call snapshot on the chain and it will keep a copy of the current
chain state mapped to a snapshot id, which is returned.
- the snapshot id always starts at 0, and increments by on every subsequent
call
- when revert is invoked, it is invoked with a snapshot_id. the chain will
reset the state to the referenced state. it will also DELETE all snapshot_id,
and associcated state for all snapshot_id >= to the snapshot_id passed in.

2. by default, anvil does not auto mine, which is a problem for confirmations. However there is a workaround.
   anvil exposes an RPC that allows you to fast-forward n blocks. See below for available RPCs.
   When waiting for confirmations > 0, we have to call the "ff" rpc to advance the blocks, to satisify 
   confirmations.

```bash

ETH_RPC_URL := env_var_or_default('ETH_RPC_URL', 'http://127.0.0.1:8545')

# Fast forward blockchain by mining blocks
# Usage: just ff [blocks]
ff blocks="10":
    @echo "⏳ Mining {{blocks}} blocks..."
    @curl -s -X POST --data '{"jsonrpc":"2.0","method":"hardhat_mine","params":[{{blocks}}],"id":1}' \
        -H "Content-Type: application/json" "{{ETH_RPC_URL}}" \
        | jq -r 'if .result == null then "✅ Successfully mined {{blocks}} blocks." else "❌ Failed: " + . end'

# Take a snapshot of the current blockchain state
# Usage: just snapshot
snapshot:
    @echo "📸 Taking snapshot..."
    @RESPONSE=$(curl -s -X POST --data '{"jsonrpc":"2.0","method":"evm_snapshot","params":[],"id":1}' \
        -H "Content-Type: application/json" "{{ETH_RPC_URL}}") && \
    SNAPSHOT_ID=$(echo "$RESPONSE" | jq -r '.result') && \
    if [ "$SNAPSHOT_ID" != "null" ]; then \
        echo "$SNAPSHOT_ID" > "$HOME/.bb_snapshot" && \
        echo "✅ Snapshot taken. ID: $SNAPSHOT_ID (Stored in $HOME/.bb_snapshot)"; \
    else \
        echo "❌ Failed to take snapshot. Response: $RESPONSE"; \
    fi

# Revert to the last snapshot and immediately take a new snapshot
# Usage: just revert [snapshot_id]
revert snapshot_id="":
    @if [ -z "{{snapshot_id}}" ] && [ -f "$HOME/.bb_snapshot" ]; then \
        SNAPSHOT_ID=$(cat "$HOME/.bb_snapshot"); \
    else \
        SNAPSHOT_ID="{{snapshot_id}}"; \
    fi && \
    if [ -z "$SNAPSHOT_ID" ]; then \
        echo "❌ No snapshot ID provided and no stored snapshot found."; \
        exit 1; \
    fi && \
    echo "🔄 Reverting to snapshot $SNAPSHOT_ID..." && \
    RESPONSE=$(curl -s -X POST --data "{\"jsonrpc\":\"2.0\",\"method\":\"evm_revert\",\"params\":[\"$SNAPSHOT_ID\"],\"id\":1}" \
        -H "Content-Type: application/json" "{{ETH_RPC_URL}}") && \
    if echo "$RESPONSE" | jq -e '.result == true' > /dev/null; then \
        echo "✅ Successfully reverted to snapshot $SNAPSHOT_ID." && \
        rm -f "$HOME/.bb_snapshot" && \
        just snapshot; \
    else \
        echo "❌ Failed to revert snapshot. Response: $RESPONSE"; \
    fi

```
