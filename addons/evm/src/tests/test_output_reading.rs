// Test that verifies we can read outputs from txtx execution

#[cfg(test)]
mod tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use txtx_addon_kit::types::types::Value;
    
    #[test]
    fn test_simple_output_reading() {
        // Create a simple runbook that just outputs values
        let runbook_content = r#"
output "test_string" {
    value = "hello world"
}

output "test_number" {
    value = 42
}

output "test_object" {
    value = {
        name = "test"
        count = 3
        active = true
    }
}
"#;
        
        let harness = ProjectTestHarness::new_with_content("test.tx", runbook_content);
        
        // Setup the project
        harness.setup().expect("Failed to setup project");
        
        // Execute the runbook
        let result = harness.execute_runbook().expect("Failed to execute runbook");
        
        assert!(result.success, "Runbook execution failed");
        
        // Check the outputs
        assert_eq!(
            harness.get_output("test_string"),
            Some(Value::String("hello world".to_string()))
        );
        
        assert_eq!(
            harness.get_output("test_number"),
            Some(Value::Integer(42))
        );
        
        // Check the object output
        if let Some(Value::Object(obj)) = harness.get_output("test_object") {
            assert_eq!(obj.get("name"), Some(&Value::String("test".to_string())));
            assert_eq!(obj.get("count"), Some(&Value::Integer(3)));
            assert_eq!(obj.get("active"), Some(&Value::Bool(true)));
        } else {
            panic!("test_object output not found or not an object");
        }
    }
}