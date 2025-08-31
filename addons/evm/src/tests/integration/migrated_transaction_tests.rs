//! Transaction tests using txtx framework with filesystem fixtures

#[cfg(test)]
mod transaction_tests {
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_simple_eth_transfer() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("💸 Testing simple ETH transfer");

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/simple_eth_transfer.tx");

        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("sender_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "ETH transfer should succeed");
                
                println!("ETH transfer completed successfully");
                
                // Verify outputs exist
                assert!(result.outputs.contains_key("tx_hash"), "Should have transaction hash");
                assert!(result.outputs.contains_key("initial_balance"), "Should have initial balance");
                assert!(result.outputs.contains_key("final_balance"), "Should have final balance");
                
                println!("   Transaction hash: {:?}", result.outputs.get("tx_hash"));
                println!("   Initial balance: {:?}", result.outputs.get("initial_balance"));
                println!("   Final balance: {:?}", result.outputs.get("final_balance"));
            }
            Err(e) => panic!("ETH transfer failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_transaction_with_custom_gas() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("⛽ Testing transaction with custom gas settings");

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/custom_gas_transfer.tx");

        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("sender_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "Transfer with custom gas should succeed");
                
                println!("Transfer with custom gas completed");
                println!("   Gas used: {:?}", result.outputs.get("gas_used"));
                println!("   Effective gas price: {:?}", result.outputs.get("effective_gas_price"));
            }
            Err(e) => panic!("Custom gas transfer failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_legacy_transaction() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("🏛️ Testing legacy transaction type");

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/legacy_transaction.tx");

        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("sender_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "Legacy transaction should succeed");
                
                println!("Legacy transaction completed");
                println!("   Transaction type: {:?}", result.outputs.get("transaction_type"));
            }
            Err(e) => panic!("Legacy transaction failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_batch_transactions() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("📦 Testing batch of transactions");

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/batch_transactions.tx");

        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("sender_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "Batch transactions should succeed");
                
                println!("Batch transactions completed");
                assert!(result.outputs.contains_key("tx1_hash"), "Should have first tx hash");
                assert!(result.outputs.contains_key("tx2_hash"), "Should have second tx hash");
                assert!(result.outputs.contains_key("tx3_hash"), "Should have third tx hash");
                
                println!("   Total gas used: {:?}", result.outputs.get("total_gas_used"));
            }
            Err(e) => panic!("Batch transactions failed: {}", e),
        }
    }
}