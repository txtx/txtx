//! Tests for Unicode string storage and retrieval in smart contracts

#[cfg(test)]
mod unicode_storage_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_unicode_storage_and_retrieval() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_unicode_storage_and_retrieval - Anvil not installed");
            return;
        }
        
        println!("Testing Unicode string storage in smart contracts");
        
        // Use fixture from filesystem
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("integration")
            .join("unicode_storage.tx");
        
        println!("Loading fixture: {}", fixture_path.display());
        
        // Read the fixture content
        let runbook_content = std::fs::read_to_string(&fixture_path)
            .expect("Failed to read unicode_storage.tx fixture");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::new_foundry("unicode_storage_test.tx", runbook_content)
            .with_anvil();
        
        // Setup project
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook
        
        
        println!("Unicode storage test completed successfully");
        
        // Verify the stored Unicode data
        if let Some(person_0) = result.outputs.get("person_0_data") {
            println!("Person 0 (with emoji): {:?}", person_0);
            // Should contain "Alice 🚀 Rocket" and 100
            let data_str = format!("{:?}", person_0);
            assert!(data_str.contains("100"), "Should contain favorite number 100");
        }
        
        if let Some(person_1) = result.outputs.get("person_1_data") {
            println!("Person 1 (Chinese): {:?}", person_1);
            // Should contain "张三" and 200
            let data_str = format!("{:?}", person_1);
            assert!(data_str.contains("200"), "Should contain favorite number 200");
        }
        
        if let Some(person_2) = result.outputs.get("person_2_data") {
            println!("Person 2 (Japanese): {:?}", person_2);
            // Should contain "田中さん" and 300
            let data_str = format!("{:?}", person_2);
            assert!(data_str.contains("300"), "Should contain favorite number 300");
        }
        
        if let Some(person_3) = result.outputs.get("person_3_data") {
            println!("Person 3 (Arabic): {:?}", person_3);
            // Should contain "مرحبا" and 400
            let data_str = format!("{:?}", person_3);
            assert!(data_str.contains("400"), "Should contain favorite number 400");
        }
        
        if let Some(person_4) = result.outputs.get("person_4_data") {
            println!("Person 4 (Mixed Unicode): {:?}", person_4);
            // Should contain mixed Unicode and 500
            let data_str = format!("{:?}", person_4);
            assert!(data_str.contains("500"), "Should contain favorite number 500");
        }
        
        // Verify name-to-number mapping works with Unicode
        if let Some(emoji_fav) = result.outputs.get("emoji_name_favorite") {
            println!("Favorite number for emoji name: {:?}", emoji_fav);
            let data_str = format!("{:?}", emoji_fav);
            assert!(data_str.contains("100"), "Emoji name should map to 100");
        }
        
        if let Some(chinese_fav) = result.outputs.get("chinese_name_favorite") {
            println!("Favorite number for Chinese name: {:?}", chinese_fav);
            let data_str = format!("{:?}", chinese_fav);
            assert!(data_str.contains("200"), "Chinese name should map to 200");
        }
        
        println!("All Unicode storage tests passed!");
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_unicode_edge_cases() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_unicode_edge_cases - Anvil not installed");
            return;
        }
        
        println!("Testing Unicode edge cases in smart contracts");
        
        // Use fixture from filesystem
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("integration")
            .join("unicode_edge_cases.tx");
        
        println!("Loading fixture: {}", fixture_path.display());
        
        // Read the fixture content
        let runbook_content = std::fs::read_to_string(&fixture_path)
            .expect("Failed to read unicode_edge_cases.tx fixture");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::new_foundry("unicode_edge_cases_test.tx", runbook_content)
            .with_anvil();
        
        // Setup project
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook
        
        
        println!("Unicode edge case test completed successfully");
        
        // Verify edge cases stored correctly
        if let Some(empty_data) = result.outputs.get("empty_string_data") {
            println!("Empty string data: {:?}", empty_data);
            let data_str = format!("{:?}", empty_data);
            assert!(data_str.contains("1"), "Empty string should have favorite number 1");
        }
        
        if let Some(long_data) = result.outputs.get("long_unicode_data") {
            println!("Long Unicode string data: {:?}", long_data);
            let data_str = format!("{:?}", long_data);
            assert!(data_str.contains("2"), "Long Unicode should have favorite number 2");
        }
        
        if let Some(special_data) = result.outputs.get("special_unicode_data") {
            println!("Special Unicode data: {:?}", special_data);
            let data_str = format!("{:?}", special_data);
            assert!(data_str.contains("3"), "Special Unicode should have favorite number 3");
        }
        
        if let Some(math_data) = result.outputs.get("math_symbols_data") {
            println!("Math symbols data: {:?}", math_data);
            let data_str = format!("{:?}", math_data);
            assert!(data_str.contains("4"), "Math symbols should have favorite number 4");
        }
        
        println!("All Unicode edge case tests passed!");
        harness.cleanup();
    }
}