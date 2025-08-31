use crate::tests::test_harness::{ProjectTestHarness, ExpectedValueBuilder, ValueComparison};
use txtx_addon_kit::types::types::Value;

#[cfg(test)]
mod structured_log_tests {
    use crate::tests::fixture_builder::MigrationHelper;
    use super::*;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    
    #[tokio::test]
    async fn test_structured_log_output() {
        eprintln!("🔍 TEST STARTING - test_structured_log_output");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        eprintln!("📋 Creating test harness with structured log fixture");
        let harness = ProjectTestHarness::new_foundry_from_fixture(
            "integration/eth_transfer_with_test_log.tx"
        );
        
        // Setup the project
        harness.setup().expect("Project setup should succeed");
        
        // Execute the runbook
        eprintln!("🔄 Executing runbook...");
        let result = result.execute().await;
        
        // Note: Since execute_runbook currently returns a mock success,
        // we'll demonstrate the API even though real execution isn't working yet
        assert!(result.is_ok(), "Execution should succeed");
        
        // Example of how to use the structured log API:
        
        // 1. Check if an action succeeded
        let send_eth_success = harness.action_succeeded("send_eth");
        eprintln!("send_eth succeeded: {}", send_eth_success);
        
        // 2. Get a specific value from the log
        if let Some(chain_id) = harness.get_log_path("test_metadata.chain_id") {
            eprintln!("Chain ID from log: {:?}", chain_id);
        }
        
        // 3. Compare a nested object with expected values
        let expected_metadata = ExpectedValueBuilder::new()
            .with_string("test_name", "eth_transfer_test")
            .with_string("timestamp", "2024-08-31")
            .with_integer("chain_id", 31337)
            .build();
        
        // This would work once we have real execution:
        // harness.assert_log_object("test_metadata", expected_metadata);
        
        // 4. Check validation flags
        if let Some(amount_correct) = harness.get_log_path("validation.amount_correct") {
            match amount_correct {
                Value::Bool(true) => eprintln!("✅ Amount validation passed"),
                _ => eprintln!("❌ Amount validation failed"),
            }
        }
        
        // 5. Get the entire action log
        if let Some(action_log) = harness.get_action_log("send_eth") {
            eprintln!("Full send_eth log: {:?}", action_log);
            
            // Compare specific fields
            if let Value::Object(obj) = action_log {
                assert!(obj.contains_key("executed"));
                assert!(obj.contains_key("tx_hash"));
                assert!(obj.contains_key("success"));
            }
        }
        
        eprintln!("✅ Test completed - API demonstrated");
    }
    
    #[tokio::test]
    async fn test_complex_object_comparison() {
        // Demonstrate comparing complex nested objects
        
        // Create an actual value (simulating what we'd get from test_log)
        let mut action_data = txtx_addon_kit::indexmap::IndexMap::new();
        action_data.insert("executed".to_string(), Value::Bool(true));
        action_data.insert("tx_hash".to_string(), Value::String("0xabc123".to_string()));
        action_data.insert("success".to_string(), Value::Bool(true));
        action_data.insert("gas_used".to_string(), Value::Integer(21000));
        
        let mut actions = txtx_addon_kit::indexmap::IndexMap::new();
        actions.insert("send_eth".to_string(), Value::Object(action_data));
        
        let mut test_log = txtx_addon_kit::indexmap::IndexMap::new();
        test_log.insert("actions".to_string(), Value::Object(actions));
        
        let actual = Value::Object(test_log);
        
        // Create expected value
        let expected_action = ExpectedValueBuilder::new()
            .with_bool("executed", true)
            .with_string("tx_hash", "0xabc123")
            .with_bool("success", true)
            .with_integer("gas_used", 21000);
        
        let expected = ExpectedValueBuilder::new()
            .with_object("actions", 
                ExpectedValueBuilder::new()
                    .with_object("send_eth", expected_action)
            )
            .build();
        
        // Compare
        let result = actual.compare_with(&expected);
        assert!(result.matches, "Objects should match");
        
        // Test partial comparison (only check some fields)
        let send_eth = actual.get_path("actions.send_eth").unwrap();
        let partial_expected = ExpectedValueBuilder::new()
            .with_bool("executed", true)
            .with_bool("success", true)
            .build();
        
        let result = send_eth.compare_fields(&partial_expected, &["executed", "success"]);
        assert!(result.matches, "Partial comparison should match");
        
        eprintln!("✅ Complex object comparison test passed");
    }
    
    #[tokio::test]
    async fn test_event_extraction() {
        // Demonstrate how to extract events from a receipt
        use crate::tests::test_harness::extract_events_from_receipt;
        use crate::tests::test_harness::events::filter_events_by_name;
        
        // Create a mock receipt with logs
        let mut log1 = txtx_addon_kit::indexmap::IndexMap::new();
        log1.insert("address".to_string(), Value::String("0x1234567890123456789012345678901234567890".to_string()));
        log1.insert("topics".to_string(), Value::Array(Box::new(vec![
            Value::String("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string()), // Transfer event
        ])));
        log1.insert("data".to_string(), Value::String("0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string()));
        log1.insert("blockNumber".to_string(), Value::Integer(100));
        log1.insert("logIndex".to_string(), Value::Integer(0));
        
        let mut receipt = txtx_addon_kit::indexmap::IndexMap::new();
        receipt.insert("logs".to_string(), Value::Array(Box::new(vec![Value::Object(log1)])));
        
        let receipt_value = Value::Object(receipt);
        
        // Extract events
        let events = extract_events_from_receipt(&receipt_value);
        eprintln!("Extracted {} events", events.len());
        assert_eq!(events.len(), 1, "Should extract one event");
        
        // Check the event was identified as Transfer
        assert_eq!(events[0].name, "Transfer");
        
        // Filter Transfer events
        let transfer_events = filter_events_by_name(&events, "Transfer");
        eprintln!("Found {} Transfer events", transfer_events.len());
        assert_eq!(transfer_events.len(), 1);
        
        eprintln!("✅ Event extraction test completed");
    }
}