#!/bin/bash

echo "=== Testing txtx doctor command ==="
echo ""

# Create a simple test case
mkdir -p /tmp/doctor_test/runbooks

cat > /tmp/doctor_test/txtx.yml << 'EOF'
name: test_project
description: Test project for doctor command

runbooks:
  transfer_test:
    location: runbooks/transfer.tx
    description: "Test transfer with output issue"
EOF

cat > /tmp/doctor_test/runbooks/transfer.tx << 'EOF'
addon "evm" {
    chain_id = "11155111"
    rpc_api_url = "https://ethereum-sepolia.publicnode.com"
}

signer "alice" "evm::wallet" {
    private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
}

action "transfer" "evm::send_eth" {
    signer = signer.alice
    recipient_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f6aE3"
    amount = 1000000000000000000
}

# These will be flagged by doctor - send_eth only outputs tx_hash!
output "sender" {
    value = action.transfer.from
}

output "receiver" {
    value = action.transfer.to
}

output "tx_result" {
    value = action.transfer.result.hash
}
EOF

echo "Created test files in /tmp/doctor_test"
echo ""
echo "Running doctor command..."
echo ""

cd /tmp/doctor_test

# Find txtx binary - use development build if available, otherwise system txtx
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
TXTX_BIN="$PROJECT_ROOT/target/debug/txtx"

if [ ! -f "$TXTX_BIN" ]; then
    TXTX_BIN="txtx"  # Fall back to system txtx
fi

"$TXTX_BIN" doctor

echo ""
echo "Note: The current implementation shows a warning because we're using a dummy manifest parser."
echo "In a full implementation, it would detect the specific issues with accessing"
echo "action.transfer.from, action.transfer.to, and action.transfer.result.hash"
echo "when send_eth only provides action.transfer.tx_hash"