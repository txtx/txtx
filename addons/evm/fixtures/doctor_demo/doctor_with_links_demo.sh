#!/bin/bash

echo "=== Enhanced Txtx Doctor with Documentation Links ==="
echo ""
echo "When doctor detects issues, it now provides direct links to documentation!"
echo ""

echo "Example problematic runbook:"
echo "----------------------------------------"
cat << 'EOF'
action "transfer" "evm::send_eth" {
    signer = signer.alice
    recipient_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f6aE3"
    amount = 1000000000000000000
}

output "from_address" {
    value = action.transfer.result.from  # ERROR!
}

output "tx_hash_from" {
    value = action.transfer.tx_hash.from  # ERROR!
}
EOF

echo ""
echo "Doctor output with documentation links:"
echo "======================================="
echo ""
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
echo "  ❌ [runbooks/example.tx:8] Invalid output access: 'evm::send_eth' action 'transfer' only provides 'tx_hash' output"
echo "     💡 Suggestion: The 'evm::send_eth' action only outputs 'tx_hash' (the transaction hash as a string)."
echo "     📚 Documentation: https://docs.txtx.sh/addons/evm/actions#send-eth"
echo ""
echo "  ❌ [runbooks/example.tx:12] Invalid output access: 'evm::send_eth' action 'transfer' only provides 'tx_hash' output"
echo "     💡 Suggestion: The 'evm::send_eth' action only outputs 'tx_hash' (the transaction hash as a string)."
echo "     📚 Documentation: https://docs.txtx.sh/addons/evm/actions#send-eth"
echo ""
echo "=== Benefits of Documentation Links ==="
echo ""
echo "1. Developers can immediately access the official documentation"
echo "2. No guessing about what outputs are available"
echo "3. Can see examples of correct usage"
echo "4. Learn about related actions (like check_confirmations)"
echo ""
echo "Other action documentation links that would be generated:"
echo "- evm::call_contract → https://docs.txtx.sh/addons/evm/actions#call-contract"
echo "- evm::deploy_contract → https://docs.txtx.sh/addons/evm/actions#deploy-contract"
echo "- stacks::call_contract → https://docs.txtx.sh/addons/stacks/actions#call-contract"
echo "- bitcoin::send_btc → https://docs.txtx.sh/addons/bitcoin/actions#send-btc"