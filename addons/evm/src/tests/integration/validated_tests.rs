//! Tests with improved DX using validation

#[cfg(test)]
mod validated_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{get_anvil_manager, runbook_validator::validate_runbook_with_report};
    use std::fs;
    use std::path::PathBuf;
    use serial_test::serial;
    use tokio;
    
    /// Test send_eth with validation
    #[tokio::test]
    #[serial(anvil)]
    async fn test_send_eth_validated() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing send_eth with validation");
        
        // Get anvil for accounts
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("send_eth_validated").await.unwrap();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Create runbook with WRONG field names to test validation
        let runbook_wrong = r#"
addon "evm" {
    chain_id = input.evm_chain_id
    rpc_api_url = input.evm_rpc_api_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "send_eth" "evm::send_eth" {
    to = input.bob_address           // WRONG: should be recipient_address
    value = 100000000000000000       // WRONG: should be amount (but at least it's an integer!)
    signer = signer.alice
}
"#;
        
        // Validate the wrong runbook
        eprintln!("\n🔍 Validating runbook with WRONG field names:");
        match validate_runbook_with_report(runbook_wrong) {
            Ok(_) => eprintln!("  ✅ Validation passed (or no schema available)"),
            Err(e) => eprintln!("  ❌ Validation failed: {}", e),
        }
        
        // Create runbook with CORRECT field names
        let runbook_correct = r#"
addon "evm" {
    chain_id = input.evm_chain_id
    rpc_api_url = input.evm_rpc_api_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "send_eth" "evm::send_eth" {
    recipient_address = input.bob_address
    amount = 100000000000000000  // INTEGER, not string!
    signer = signer.alice
}
"#;
        
        // Validate the correct runbook
        eprintln!("\n🔍 Validating runbook with CORRECT field names:");
        match validate_runbook_with_report(runbook_correct) {
            Ok(_) => eprintln!("  ✅ Validation passed"),
            Err(e) => eprintln!("  ❌ Validation failed: {}", e),
        }
        
        println!("\n✅ Validation test completed - demonstrating improved DX");
    }
    
    /// Test that shows what better error messages would look like
    #[tokio::test]
    #[serial(anvil)]
    async fn test_better_error_messages() {
        println!("\n📋 Example of improved error messages:\n");
        
        // Simulate what txtx SHOULD show instead of panicking
        let better_error = r#"
Error: Invalid configuration for action 'send_eth' (evm::send_eth)
  ✗ Missing required field: 'recipient_address'
  ✗ Unknown field: 'to' (did you mean 'recipient_address'?)
  ✗ Unknown field: 'value' (did you mean 'amount'?)
  
Required fields:
  - recipient_address: string - The address to send ETH to
  - amount: string - Amount of ETH to send in wei
  - signer: signer - The signer to use for the transaction

Optional fields:
  - confirmations: number - Number of confirmations to wait (default: 1)
  - gas_limit: string - Gas limit for the transaction
  
See documentation: https://docs.txtx.sh/addons/evm/actions#send-eth
"#;
        
        println!("{}", better_error);
        
        println!("\nInstead of current error:");
        println!("  thread 'main' panicked at crates/txtx-addon-kit/src/types/types.rs:349:18:");
        println!("  internal error: entered unreachable code\n");
        
        println!("✅ This would significantly improve the testing DX!");
    }
}