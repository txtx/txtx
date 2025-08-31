//! Test to validate that all runbook fixtures are valid and loadable

#[cfg(test)]
mod fixture_validation {
    use std::fs;
    use std::path::Path;
    
    #[test]
    fn test_all_fixtures_are_valid_runbooks() {
        let fixtures_dir = Path::new("src/tests/fixtures/runbooks");
        
        // List of all fixture files
        let fixtures = vec![
            "errors/insufficient_funds.tx",
            "errors/missing_config_field.tx", 
            "errors/function_not_found.tx",
            "errors/signer_key_not_found.tx",
            "codec/invalid_hex.tx",
        ];
        
        for fixture_path in fixtures {
            let full_path = fixtures_dir.join(fixture_path);
            
            // Check file exists
            assert!(
                full_path.exists(),
                "Fixture file not found: {}",
                full_path.display()
            );
            
            // Read and validate content
            let content = fs::read_to_string(&full_path)
                .expect(&format!("Failed to read fixture: {}", fixture_path));
            
            // Basic validation - ensure it has required sections
            assert!(
                content.contains("addon \"evm\""),
                "Fixture {} missing addon section",
                fixture_path
            );
            
            // Check for action or function (at least one should be present)
            let has_action = content.contains("action ");
            let has_function = content.contains("function ");
            assert!(
                has_action || has_function,
                "Fixture {} has neither action nor function",
                fixture_path
            );
            
            println!("✓ Validated fixture: {}", fixture_path);
        }
    }
    

}