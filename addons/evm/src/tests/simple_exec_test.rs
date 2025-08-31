// Simple test to debug execution

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;
    
    #[test]
    fn test_simple_execution() {
        // Create a temp directory
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create a simple runbook
        fs::create_dir_all(project_path.join("runbooks")).unwrap();
        fs::write(
            project_path.join("runbooks/test.tx"),
            r#"
output "test_value" {
    value = "hello"
}

output "test_number" {
    value = 42
}
"#
        ).unwrap();
        
        // Create txtx.yml
        fs::write(
            project_path.join("txtx.yml"),
            r#"
name: test-project
version: 1.0.0

environments:
  test:
    description: Test environment
"#
        ).unwrap();
        
        // Build txtx binary
        let txtx_binary = {
            let build_output = Command::new("cargo")
                .arg("build")
                .arg("--package")
                .arg("txtx-cli")
                .current_dir(std::env!("CARGO_MANIFEST_DIR").to_string() + "/../..")
                .output()
                .unwrap();
            
            assert!(build_output.status.success(), "Failed to build txtx");
            
            std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
                .parent().unwrap()
                .parent().unwrap()
                .join("target/debug/txtx")
        };
        
        // Run txtx with JSON output
        let output = Command::new(&txtx_binary)
            .arg("run")
            .arg("runbooks/test.tx")
            .arg("--env")
            .arg("test")
            .arg("--output-json")
            .arg("--unsupervised")
            .current_dir(&project_path)
            .output()
            .unwrap();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        println!("Exit status: {:?}", output.status.code());
        println!("STDOUT:\n{}", stdout);
        println!("STDERR:\n{}", stderr);
        
        assert!(output.status.success(), "txtx execution failed");
        
        // Parse the JSON output
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Failed to parse JSON output");
        
        println!("Parsed JSON: {:#?}", json);
        
        // Check outputs
        assert_eq!(json["outputs"]["test_value"], "hello");
        assert_eq!(json["outputs"]["test_number"], 42);
    }
}