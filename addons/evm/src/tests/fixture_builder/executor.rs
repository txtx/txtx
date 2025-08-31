// Executor for running txtx runbooks in test fixtures

use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use serde_json::Value as JsonValue;
use txtx_addon_kit::types::types::Value;

/// Result from executing a runbook
#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub outputs: HashMap<String, Value>,
    pub output_file: PathBuf,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a txtx runbook via CLI
pub fn execute_runbook(
    project_dir: &Path,
    runbook_name: &str,
    environment: &str,
    inputs: &HashMap<String, String>,
) -> Result<ExecutionResult, Box<dyn std::error::Error>> {
    eprintln!("🚀 Executing runbook: {}", runbook_name);
    
    // Build txtx binary path
    let txtx_binary = find_txtx_binary()?;
    
    // Build the command
    let mut cmd = Command::new(&txtx_binary);
    cmd.arg("run")
       .arg(runbook_name)
       .arg("--env")
       .arg(environment)
       .arg("--output-json")
       .arg("runs")
       .arg("-u")  // unsupervised
       .current_dir(project_dir);
    
    // Add inputs
    for (key, value) in inputs {
        cmd.arg("--input")
           .arg(format!("{}={}", key, value));
    }
    
    eprintln!("  📝 Command: {:?}", cmd);
    
    // Execute
    let output = cmd.output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if !output.status.success() {
        eprintln!("  ❌ Execution failed:");
        eprintln!("    Exit code: {:?}", output.status.code());
        eprintln!("    Stderr: {}", stderr);
        return Ok(ExecutionResult {
            success: false,
            outputs: HashMap::new(),
            output_file: PathBuf::new(),
            stdout,
            stderr,
        });
    }
    
    // Find the output file
    let output_file = find_latest_output_file(project_dir, environment, runbook_name)?;
    eprintln!("  📄 Output file: {}", output_file.display());
    
    // Parse outputs
    let outputs = parse_output_file(&output_file)?;
    eprintln!("  ✅ Execution successful, {} outputs captured", outputs.len());
    
    Ok(ExecutionResult {
        success: true,
        outputs,
        output_file,
        stdout,
        stderr,
    })
}

/// Build the txtx binary from source
/// This ensures we're always testing the current code, not some old artifact
fn find_txtx_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Always build from source to ensure we're testing current code
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();
    
    eprintln!("  🔨 Building txtx-cli from source...");
    eprintln!("    Workspace: {}", workspace_root.display());
    
    let build_output = Command::new("cargo")
        .arg("build")
        .arg("--package")
        .arg("txtx-cli")
        .arg("--bin")
        .arg("txtx")
        .current_dir(&workspace_root)
        .output()?;
    
    if !build_output.status.success() {
        eprintln!("    ❌ Build failed:");
        eprintln!("    Stderr: {}", String::from_utf8_lossy(&build_output.stderr));
        return Err(format!(
            "Failed to build txtx-cli: {}",
            String::from_utf8_lossy(&build_output.stderr)
        ).into());
    }
    
    let binary_path = workspace_root.join("target/debug/txtx");
    
    if !binary_path.exists() {
        return Err(format!(
            "Built txtx binary not found at expected location: {}",
            binary_path.display()
        ).into());
    }
    
    eprintln!("    ✅ Built txtx binary: {}", binary_path.display());
    Ok(binary_path)
}

/// Find the latest output file
fn find_latest_output_file(
    project_dir: &Path,
    environment: &str,
    runbook_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use std::fs;
    use std::time::SystemTime;
    
    let output_dir = project_dir.join("runs").join(environment);
    
    if !output_dir.exists() {
        return Err(format!("Output directory not found: {}", output_dir.display()).into());
    }
    
    // Find files matching pattern
    let mut matching_files: Vec<_> = fs::read_dir(&output_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(runbook_name) && name_str.ends_with(".output.json")
        })
        .collect();
    
    if matching_files.is_empty() {
        return Err(format!("No output file found for runbook: {}", runbook_name).into());
    }
    
    // Sort by modification time
    matching_files.sort_by_key(|entry| {
        entry.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    
    Ok(matching_files.last().unwrap().path())
}

/// Parse the output JSON file
fn parse_output_file(path: &Path) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    use std::fs;
    
    let content = fs::read_to_string(path)?;
    let json: JsonValue = serde_json::from_str(&content)?;
    
    let mut outputs = HashMap::new();
    
    if let JsonValue::Object(obj) = json {
        for (key, value) in obj {
            // Handle nested { "value": ... } structure
            let actual_value = if let Some(inner) = value.get("value") {
                json_to_txtx_value(inner)
            } else {
                json_to_txtx_value(&value)
            };
            outputs.insert(key, actual_value);
        }
    }
    
    Ok(outputs)
}

/// Convert JSON to txtx Value
fn json_to_txtx_value(json: &JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i as i128)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        },
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => {
            Value::Array(Box::new(arr.iter().map(json_to_txtx_value).collect()))
        },
        JsonValue::Object(obj) => {
            use txtx_addon_kit::indexmap::IndexMap;
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_txtx_value(v));
            }
            Value::Object(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_json_conversion() {
        let json = serde_json::json!({
            "string": "hello",
            "number": 42,
            "bool": true,
            "null": null,
            "array": [1, 2, 3],
            "object": {
                "nested": "value"
            }
        });
        
        let value = json_to_txtx_value(&json);
        
        match value {
            Value::Object(map) => {
                assert_eq!(map.get("string"), Some(&Value::String("hello".to_string())));
                assert_eq!(map.get("number"), Some(&Value::Integer(42)));
                assert_eq!(map.get("bool"), Some(&Value::Bool(true)));
                assert_eq!(map.get("null"), Some(&Value::Null));
                
                if let Some(Value::Array(arr)) = map.get("array") {
                    assert_eq!(arr.len(), 3);
                }
                
                if let Some(Value::Object(nested)) = map.get("object") {
                    assert_eq!(nested.get("nested"), Some(&Value::String("value".to_string())));
                }
            },
            _ => panic!("Expected object"),
        }
    }
}