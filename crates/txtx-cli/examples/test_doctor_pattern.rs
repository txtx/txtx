// This demonstrates testing the pattern detection logic

fn main() {
    // Sample runbook content with the problematic pattern
    let problematic_content = r#"
addon "evm" {
    chain_id = "11155111"
    rpc_api_url = "https://ethereum-sepolia.publicnode.com"
}

action "transfer" "evm::send_eth" {
    signer = signer.alice
    recipient_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f6aE3"
    amount = 1000000000000000000
}

# These patterns will be detected as errors
output "from" {
    value = action.transfer.result.from
}

output "to" {
    value = action.transfer.to
}

output "amount" {
    value = action.transfer.value
}
"#;

    println!("Testing pattern detection for send_eth output issues...\n");
    
    // The patterns our doctor command looks for
    let patterns = [
        "action.transfer.result",
        "action.transfer.from", 
        "action.transfer.to",
        "action.transfer.value"
    ];
    
    for pattern in &patterns {
        if problematic_content.contains(pattern) {
            println!("❌ FOUND ISSUE: Trying to access '{}' when send_eth only provides 'tx_hash'", pattern);
            println!("   Suggestion: Use 'action.transfer.tx_hash' and 'evm::get_transaction' for details\n");
        }
    }
    
    println!("\nThe doctor command implements this pattern detection");
    println!("and would have caught your exact issue immediately!");
}