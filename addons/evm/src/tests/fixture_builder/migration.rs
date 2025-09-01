// Migration helpers for converting integration tests to FixtureBuilder

use super::FixtureBuilder;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use txtx_addon_kit::types::types::Value;

/// Helper to migrate from ProjectTestHarness pattern to FixtureBuilder
pub struct MigrationHelper {
    fixture_path: PathBuf,
    inputs: HashMap<String, String>,
}

impl MigrationHelper {
    /// Create a migration helper from a fixture path
    pub fn from_fixture(fixture_path: &Path) -> Self {
        Self {
            fixture_path: fixture_path.to_path_buf(),
            inputs: HashMap::new(),
        }
    }
    
    /// Add an input parameter
    pub fn with_input(mut self, key: &str, value: &str) -> Self {
        self.inputs.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Build and execute the fixture
    pub async fn execute(self) -> Result<TestResult, Box<dyn std::error::Error>> {
        // Extract test name from fixture path
        let test_name = self.fixture_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");
        
        // Read the fixture content
        let fixture_content = fs::read_to_string(&self.fixture_path)?;
        
        // Build the fixture with inputs
        let mut builder = FixtureBuilder::new(test_name)
            .with_runbook("main", &fixture_content);
        
        // Add all inputs as parameters
        for (key, value) in self.inputs {
            builder = builder.with_parameter(&key, &value);
        }
        
        // Build and execute
        let mut fixture = builder.build().await?;
        fixture.execute_runbook("main").await?;
        
        // Extract outputs
        let outputs = fixture.get_outputs("main")
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), value_to_json(v)))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(TestResult {
            success: true,
            outputs,
            error: None,
        })
    }
}

/// Convert Value to serde_json::Value
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::json!(f),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        },
        Value::Object(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        },
        Value::Null => serde_json::Value::Null,
        _ => serde_json::Value::String(format!("{:?}", value)),
    }
}

/// Result from test execution
#[derive(Debug)]
pub struct TestResult {
    pub success: bool,
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}

/// Simple execution helper for common test patterns
pub async fn execute_fixture(
    fixture_name: &str,
    inputs: HashMap<&str, &str>,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/integration")
        .join(format!("{}.tx", fixture_name));
    
    let mut helper = MigrationHelper::from_fixture(&fixture_path);
    for (key, value) in inputs {
        helper = helper.with_input(key, value);
    }
    
    helper.execute().await
}

/// Execute a fixture with no inputs
pub async fn execute_simple_fixture(
    fixture_name: &str,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    execute_fixture(fixture_name, HashMap::new()).await
}