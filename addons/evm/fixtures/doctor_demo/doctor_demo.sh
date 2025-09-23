#!/bin/bash

echo "=== Txtx Doctor Command Demo ==="
echo ""
echo "This demonstrates how 'txtx doctor' would help catch the send_eth output issue"
echo "that cost us 2+ hours of debugging."
echo ""

# Create a test directory
TEST_DIR="/tmp/txtx_doctor_demo"
rm -rf $TEST_DIR
mkdir -p $TEST_DIR/runbooks

# Create txtx.yml
cat > $TEST_DIR/txtx.yml << 'EOF'
name: doctor_demo
description: Demonstrates txtx doctor finding common issues

runbooks:
  problematic:
    location: runbooks/problematic.tx
    description: "Has the send_eth output access issue"
EOF

# Create problematic runbook
cat > $TEST_DIR/runbooks/problematic.tx << 'EOF'
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

# THIS WILL CAUSE AN ERROR - send_eth only outputs tx_hash!
output "from_address" {
    value = action.transfer.result.from
}

output "to_address" {
    value = action.transfer.result.to
}
EOF

echo "Created test files in $TEST_DIR"
echo ""
echo "Running: txtx doctor --manifest-file-path $TEST_DIR/txtx.yml"
echo ""

# Show what the doctor command would output
echo "🏥 Txtx Doctor Results"
echo ""
echo "📊 Summary:"
echo "   Runbooks checked: 1"
echo "   Actions validated: 1"
echo "   Outputs validated: 2"
echo ""
echo "📋 Issues found:"
echo "   ❌ Errors: 2"
echo "   ⚠️  Warnings: 0"
echo "   ℹ️  Info: 0"
echo ""
echo "📤 Output Validation Issues (2 issues):"
echo ""
echo "  ❌ [runbooks/problematic.tx:19] Invalid output access: 'send_eth' action 'transfer' only provides 'tx_hash' output"
echo "     💡 Suggestion: To get transaction details, use 'evm::get_transaction' with the tx_hash"
echo "     📝 Example:"
echo "        # Store values before the transaction"
echo "        variable \"sender_address\" {"
echo "            value = signer.alice.address"
echo "        }"
echo ""
echo "        action \"transfer\" \"evm::send_eth\" {"
echo "            signer = signer.alice"
echo "            recipient_address = var.recipient"
echo "            amount = var.amount"
echo "        }"
echo ""
echo "        output \"from_address\" {"
echo "            value = var.sender_address  # Use stored value"
echo "        }"
echo ""
echo "  ❌ [runbooks/problematic.tx:23] Invalid output access: 'send_eth' action 'transfer' only provides 'tx_hash' output"
echo "     💡 Suggestion: To get transaction details, use 'evm::get_transaction' with the tx_hash"
echo ""
echo "=== Without txtx doctor ==="
echo "Developer would see: 'DependencyNotComputed' and spend 2+ hours debugging"
echo ""
echo "=== With txtx doctor ==="
echo "Developer immediately knows:"
echo "1. send_eth only outputs tx_hash"
echo "2. How to get the full transaction details"
echo "3. Example code to fix the issue"