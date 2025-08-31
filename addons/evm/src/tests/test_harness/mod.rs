//! Project-based test harness for EVM addon tests
//! 
//! This module provides a complete txtx project environment for testing,
//! supporting both Foundry and Hardhat compilation outputs.

pub mod assertions;
pub mod events;

pub use assertions::{ValueComparison, ComparisonResult, ExpectedValueBuilder};
pub use events::{ParsedEvent, extract_events_from_receipt};

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::collections::HashMap;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::types::Value;
use super::integration::anvil_harness::AnvilInstance;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::errors::{EvmError, TransactionError, RpcError, ContractError, CodecError, ConfigError};
use error_stack::Report;

// Imports for txtx-core integration
use txtx_addon_kit::Addon;
use txtx_addon_kit::types::{AuthorizationContext, RunbookId};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_core::{
    runbook::{Runbook, RunbookTopLevelInputsMap, RuntimeContext},
    types::RunbookSources,
    start_unsupervised_runbook_runloop,
};
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

// Module organization

// Tests for the harness itself
#[cfg(test)]
mod tests;

// Integration tests that use the harness
#[cfg(test)]
mod integration_tests;

/// Compilation framework to use for the test project
#[derive(Debug, Clone)]
pub enum CompilationFramework {
    Foundry,
    Hardhat,
}

/// Helper to convert JSON value to txtx Value
fn json_to_txtx_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i as i128)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        },
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::Array(Box::new(arr.iter().map(json_to_txtx_value).collect()))
        },
        serde_json::Value::Object(obj) => {
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_txtx_value(v));
            }
            Value::Object(map)
        }
    }
}

/// Addon provider function for tests
fn get_test_addon_by_namespace(namespace: &str) -> Option<Box<dyn Addon>> {
    match namespace {
        "evm" => Some(Box::new(crate::EvmNetworkAddon::new())),
        "std" => Some(Box::new(txtx_test_utils::StdAddon::new())),
        _ => None,
    }
}

/// A complete txtx project environment for testing
pub struct ProjectTestHarness {
    /// Temporary directory containing the project
    pub temp_dir: TempDir,
    /// Path to the project root
    pub project_path: PathBuf,
    /// Compilation framework being used
    pub framework: CompilationFramework,
    /// Inputs to pass to the runbook (--input key=value)
    pub inputs: HashMap<String, String>,
    /// The runbook content to test
    pub runbook_content: String,
    /// Name of the runbook file
    pub runbook_name: String,
    /// Optional Anvil instance for blockchain testing
    pub anvil: Option<AnvilInstance>,
    /// Flag to indicate if test failed (for preserving temp dir during migration)
    test_failed: Arc<AtomicBool>,
}

impl ProjectTestHarness {
    /// Create a new test project with Foundry
    pub fn new_foundry(runbook_name: &str, runbook_content: String) -> Self {
        Self::new(runbook_name, runbook_content, CompilationFramework::Foundry)
    }

    /// Create a new test project with Hardhat
    pub fn new_hardhat(runbook_name: &str, runbook_content: String) -> Self {
        Self::new(runbook_name, runbook_content, CompilationFramework::Hardhat)
    }
    
    /// Create a new test project with Hardhat from fixture
    pub fn new_hardhat_from_fixture(fixture_name: &str) -> Self {
        let fixture_path = Self::fixture_path(fixture_name);
        let runbook_content = Self::read_fixture(&fixture_path);
        let runbook_name = Path::new(fixture_name)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        Self::new_hardhat(runbook_name, runbook_content)
    }

    /// Create from a fixture file path
    pub fn new_foundry_from_fixture(fixture_name: &str) -> Self {
        let fixture_path = Self::fixture_path(fixture_name);
        let runbook_content = Self::read_fixture(&fixture_path);
        let runbook_name = Path::new(fixture_name)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        Self::new_foundry(runbook_name, runbook_content)
    }
    
    /// Create a new test harness with custom content
    pub fn new_with_content(runbook_name: &str, content: &str) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_path = temp_dir.path().to_path_buf();
        
        Self {
            project_path,
            runbook_name: runbook_name.to_string(),
            runbook_content: content.to_string(),
            framework: CompilationFramework::Foundry,
            inputs: HashMap::new(),
            anvil: None,
            temp_dir,
            test_failed: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Create from an existing fixture file
    pub fn from_fixture(fixture_path: &Path) -> Self {
        let runbook_content = fs::read_to_string(fixture_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path.display(), e));
        
        let runbook_name = fixture_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test.tx")
            .to_string();
        
        Self::new_foundry(&runbook_name, runbook_content)
    }
    
    /// Get the base path for fixtures
    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name)
    }
    
    /// Read a fixture file
    fn read_fixture(path: &Path) -> String {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            Self::fixture_path(path.to_str().unwrap())
        };
        
        fs::read_to_string(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", full_path.display(), e))
    }

    /// Create a new test project
    fn new(runbook_name: &str, runbook_content: String, framework: CompilationFramework) -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_path = temp_dir.path().to_path_buf();

        Self {
            temp_dir,
            project_path,
            framework,
            inputs: HashMap::new(),
            runbook_content,
            runbook_name: runbook_name.to_string(),
            anvil: None,
            test_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add an input to pass to the runbook
    pub fn with_input(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inputs.insert(key.into(), value.into());
        self
    }
    
    /// Enable Anvil for this test
    pub fn with_anvil(mut self) -> Self {
        if !AnvilInstance::is_available() {
            panic!("Anvil not found. Please install Foundry: curl -L https://foundry.paradigm.xyz | bash");
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        // Automatically add Anvil configuration as inputs for testing environment
        self.inputs.insert("rpc_url".to_string(), anvil.url.clone());
        self.inputs.insert("chain_id".to_string(), anvil.chain_id.to_string());
        
        // Add test accounts as inputs - matching the names expected by fixtures
        if !anvil.accounts.is_empty() {
            let account0 = &anvil.accounts[0];
            self.inputs.insert("sender_address".to_string(), format!("{:?}", account0.address));
            self.inputs.insert("sender_private_key".to_string(), account0.private_key.clone());
            self.inputs.insert("private_key".to_string(), account0.private_key.clone()); // Common alias
            self.inputs.insert("deployer_private_key".to_string(), account0.private_key.clone());
            
            if anvil.accounts.len() > 1 {
                let account1 = &anvil.accounts[1];
                self.inputs.insert("recipient_address".to_string(), format!("{:?}", account1.address));
                self.inputs.insert("recipient".to_string(), format!("{:?}", account1.address)); // Common alias
                self.inputs.insert("recipient_private_key".to_string(), account1.private_key.clone());
            }
            
            // Add default amount for simple transfer tests
            self.inputs.insert("amount".to_string(), "1000000000000000000".to_string()); // 1 ETH in wei
        }
        
        self.anvil = Some(anvil);
        self
    }

    /// Get the project path
    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    /// Setup the project structure based on the framework
    pub fn setup(&self) -> Result<(), String> {
        // Create project directories
        fs::create_dir_all(self.project_path.join("runbooks"))
            .map_err(|e| format!("Failed to create runbooks directory: {}", e))?;

        // Write the runbook
        let runbook_path = self.project_path.join("runbooks").join(&self.runbook_name);
        fs::write(&runbook_path, &self.runbook_content)
            .map_err(|e| format!("Failed to write runbook: {}", e))?;

        // Create txtx.yml configuration
        let txtx_config = self.generate_txtx_config();
        fs::write(self.project_path.join("txtx.yml"), txtx_config)
            .map_err(|e| format!("Failed to write txtx.yml: {}", e))?;

        // Setup framework-specific files
        match self.framework {
            CompilationFramework::Foundry => self.setup_foundry_project()?,
            CompilationFramework::Hardhat => self.setup_hardhat_project()?,
        }

        Ok(())
    }

    /// Generate txtx.yml configuration
    fn generate_txtx_config(&self) -> String {
        let mut config = String::from("---\n");
        config.push_str("name: test-project\n");
        config.push_str("id: test-project\n");
        config.push_str("runbooks:\n");
        config.push_str(&format!("  - name: {}\n", self.runbook_name.trim_end_matches(".tx")));
        config.push_str(&format!("    id: {}\n", self.runbook_name.trim_end_matches(".tx")));
        config.push_str("    description: Test runbook\n");
        config.push_str(&format!("    location: runbooks/{}\n", self.runbook_name));
        config.push_str("environments:\n");
        config.push_str("  global:\n");
        config.push_str("    confirmations: 0\n");
        config.push_str("  testing:\n");  // Using 'testing' to match the command
        config.push_str("    confirmations: 0\n");
        
        // Add inputs as environment variables if they're not passed via --input
        if !self.inputs.is_empty() {
            for (key, value) in &self.inputs {
                config.push_str(&format!("      {}: \"{}\"\n", key, value));
            }
        }
        
        config
    }

    /// Setup Foundry-specific project files
    fn setup_foundry_project(&self) -> Result<(), String> {
        // Create src directory for contracts
        let src_dir = self.project_path.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| format!("Failed to create src directory: {}", e))?;
        
        // Create the SimpleStorage contract
        let simple_storage_contract = r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract SimpleStorage {
    uint256 private storedData;
    
    event DataStored(uint256 data);
    
    constructor(uint256 initialValue) {
        storedData = initialValue;
        emit DataStored(initialValue);
    }
    
    function set(uint256 x) public {
        storedData = x;
        emit DataStored(x);
    }
    
    function retrieve() public view returns (uint256) {
        return storedData;
    }
}"#;
        fs::write(src_dir.join("SimpleStorage.sol"), simple_storage_contract)
            .map_err(|e| format!("Failed to write SimpleStorage.sol: {}", e))?;

        // Create foundry.toml
        let foundry_config = r#"[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc = "0.8.20"
"#;
        fs::write(self.project_path.join("foundry.toml"), foundry_config)
            .map_err(|e| format!("Failed to write foundry.toml: {}", e))?;
        
        // Try to run forge build if available
        let forge_result = std::process::Command::new("forge")
            .arg("build")
            .current_dir(&self.project_path)
            .output();
        
        match forge_result {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully compiled contracts with forge");
            },
            Ok(output) => {
                eprintln!("Warning: forge build failed: {}", String::from_utf8_lossy(&output.stderr));
                // Create a minimal artifact if forge fails
                self.create_minimal_artifacts()?;
            },
            Err(_) => {
                eprintln!("Warning: forge not found, creating minimal artifacts");
                // Create a minimal artifact if forge is not available
                self.create_minimal_artifacts()?;
            }
        }

        Ok(())
    }
    
    /// Create minimal artifacts for testing when forge is not available
    fn create_minimal_artifacts(&self) -> Result<(), String> {
        fs::create_dir_all(self.project_path.join("out/SimpleStorage.sol"))
            .map_err(|e| format!("Failed to create out directory: {}", e))?;
        
        // Minimal but valid artifact
        let simple_storage_artifact = r#"{
  "abi": [
    {
      "inputs": [{"internalType": "uint256", "name": "initialValue", "type": "uint256"}],
      "stateMutability": "nonpayable",
      "type": "constructor"
    },
    {
      "inputs": [],
      "name": "retrieve",
      "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
      "stateMutability": "view",
      "type": "function"
    }
  ],
  "bytecode": {
    "object": "0x608060405234801561001057600080fd5b5060405161016f38038061016f833981016040819052610030916100537565b600081905560405181815233907f91a12cb8680d2fae77e047f9dd9dd0adc3475390beb7c57e82bb26db65ced8d79060200160405180910390a25061006b565b60006020828403121561006557600080fd5b5051919050565b60f58061007a6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80632e64cec11460375780636057361d14604f575b600080fd5b60005460405190151581526020015b60405180910390f35b605c605a366004605e565b005b005b600060208284031215606f57600080fd5b503591905056fea264697066735822122043ca9ef891c2d8c19b2f270801c478a44969328fe7b2fa1c7c1f3f94f96cbcd564736f6c63430008140033"
  }
}"#;
        fs::write(
            self.project_path.join("out/SimpleStorage.sol/SimpleStorage.json"),
            simple_storage_artifact
        ).map_err(|e| format!("Failed to write SimpleStorage artifact: {}", e))?;
        
        Ok(())
    }

    /// Setup Hardhat-specific project files
    fn setup_hardhat_project(&self) -> Result<(), String> {
        // Create artifacts directories
        fs::create_dir_all(self.project_path.join("artifacts/contracts/SimpleStorage.sol"))
            .map_err(|e| format!("Failed to create artifacts directory: {}", e))?;

        // Copy or create Hardhat artifacts
        // TODO: Add SimpleStorage.json fixture
        let simple_storage_artifact = r#"{"abi":[],"bytecode":"0x"}"#;
        fs::write(
            self.project_path.join("artifacts/contracts/SimpleStorage.sol/SimpleStorage.json"),
            simple_storage_artifact
        ).map_err(|e| format!("Failed to write SimpleStorage artifact: {}", e))?;

        // Create hardhat.config.js
        let hardhat_config = r#"module.exports = {
  solidity: "0.8.20",
  networks: {
    localhost: {
      url: "http://127.0.0.1:8545"
    }
  }
};
"#;
        fs::write(self.project_path.join("hardhat.config.js"), hardhat_config)
            .map_err(|e| format!("Failed to write hardhat.config.js: {}", e))?;

        Ok(())
    }

    /// Execute runbook and determine success/failure from the Report
    /// Outputs can be read from temp folder after execution
    pub fn execute_runbook(&self) -> Result<TestResult, Report<EvmError>> {
        // For now, just return a simple success to verify the structure works
        // The real implementation would execute txtx and read state from temp folder
        
        eprintln!("execute_runbook: Starting actual execution");
        
        // Actually execute the runbook via CLI
        self.execute_runbook_via_cli()
    }
    
    /// Old CLI approach - kept for reference but not used
    pub fn execute_runbook_via_cli(&self) -> Result<TestResult, Report<EvmError>> {
        use std::process::Command;
        use serde_json::Value as JsonValue;
        use std::path::PathBuf;
        
        eprintln!("execute_runbook: Executing via CLI with JSON output");
        
        // First, ensure txtx binary is built
        let txtx_binary = {
            // Try to find existing binary first
            let possible_paths = vec![
                PathBuf::from("target/debug/txtx"),
                PathBuf::from("target/release/txtx"),
                // From the workspace root
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent().unwrap()
                    .parent().unwrap()
                    .join("target/debug/txtx"),
            ];
            
            let mut found = None;
            for path in possible_paths {
                if path.exists() {
                    found = Some(path);
                    break;
                }
            }
            
            if let Some(path) = found {
                path
            } else {
                // Build it if not found
                eprintln!("Building txtx binary...");
                let build_output = Command::new("cargo")
                    .arg("build")
                    .arg("--package")
                    .arg("txtx-cli")
                    .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .parent().unwrap()
                        .parent().unwrap())
                    .output()
                    .map_err(|e| Report::new(EvmError::Config(
                        ConfigError::ParseError(format!("Failed to build txtx: {}", e))
                    )))?;
                    
                if !build_output.status.success() {
                    return Err(Report::new(EvmError::Config(
                        ConfigError::ParseError(format!("Failed to build txtx: {}", 
                            String::from_utf8_lossy(&build_output.stderr)))
                    )));
                }
                
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent().unwrap()
                    .parent().unwrap()
                    .join("target/debug/txtx")
            }
        };
        
        eprintln!("Using txtx binary: {}", txtx_binary.display());
        
        // Create output directory for JSON
        let output_dir = self.project_path.join("runs");
        fs::create_dir_all(&output_dir)
            .map_err(|e| Report::new(EvmError::Config(
                ConfigError::ParseError(format!("Failed to create output directory: {}", e))
            )))?;
        
        // Build the txtx command
        let mut cmd = Command::new(txtx_binary);
        cmd.arg("run")
           .arg(self.runbook_name.trim_end_matches(".tx"))  // Just the runbook name without extension
           .arg("--env")
           .arg("testing")  // Changed to 'testing' to match the environment name
           .arg("--output-json")
           .arg(output_dir.to_str().unwrap())  // Specify output directory
           .arg("-u")  // Short form for unsupervised
           .current_dir(&self.project_path);
        
        // Add all inputs as command line arguments
        for (key, value) in &self.inputs {
            cmd.arg("--input")
               .arg(format!("{}={}", key, value));
        }
        
        eprintln!("Running command: {:?}", cmd);
        
        // Execute the command
        let output = cmd.output().map_err(|e| {
            Report::new(EvmError::Config(
                ConfigError::ParseError(format!("Failed to execute txtx: {}", e))
            ))
        })?;
        
        // Check if execution was successful
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            eprintln!("txtx execution failed!");
            eprintln!("Exit code: {:?}", output.status.code());
            eprintln!("STDERR: {}", stderr);
            eprintln!("STDOUT: {}", stdout);
            return Err(Report::new(EvmError::Config(
                ConfigError::ParseError(format!("txtx execution failed: {}", stderr))
            )));
        }
        
        // Find the output JSON file in runs/testing/
        let runs_dir = self.project_path.join("runs/testing");
        let mut outputs = HashMap::new();
        
        if runs_dir.exists() {
            // Find the most recent output file
            let mut output_files: Vec<_> = fs::read_dir(&runs_dir)
                .map_err(|e| Report::new(EvmError::Config(
                    ConfigError::ParseError(format!("Failed to read runs directory: {}", e))
                )))?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry.path().extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .collect();
            
            // Sort by modification time to get the most recent
            output_files.sort_by_key(|entry| {
                entry.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            
            if let Some(latest_file) = output_files.last() {
                let output_file = latest_file.path();
                eprintln!("Reading output from: {}", output_file.display());
                
                let json_content = fs::read_to_string(&output_file).map_err(|e| {
                    Report::new(EvmError::Config(
                        ConfigError::ParseError(format!("Failed to read output file: {}", e))
                    ))
                })?;
                
                eprintln!("Output file content: {}", json_content);
                
                let json: JsonValue = serde_json::from_str(&json_content).map_err(|e| {
                    Report::new(EvmError::Config(
                        ConfigError::ParseError(format!("Failed to parse JSON output: {}. Content was: {}", e, json_content))
                    ))
                })?;
                
                // Extract outputs from JSON
                if let Some(outputs_obj) = json.as_object() {
                    for (key, value) in outputs_obj {
                        // Handle nested value structure {"value": ...}
                        let txtx_value = if let Some(inner_value) = value.get("value") {
                            json_to_txtx_value(inner_value)
                        } else {
                            json_to_txtx_value(value)
                        };
                        outputs.insert(key.clone(), txtx_value);
                    }
                }
            } else {
                eprintln!("Warning: No output files found in {}", runs_dir.display());
            }
        } else {
            eprintln!("Warning: Runs directory not found at {}", runs_dir.display());
            eprintln!("STDOUT output was: {}", String::from_utf8_lossy(&output.stdout));
        }
        
        eprintln!("Parsed outputs: {:?}", outputs);
        
        Ok(TestResult {
            success: true,
            outputs,
            error: None,
        })
    }
    
    /// Execute the runbook using txtx-core unsupervised mode (async version)
    pub async fn execute_runbook_async(&self) -> Result<TestResult, Report<EvmError>> {
        eprintln!("execute_runbook_async: Starting execution of runbook: {}", self.runbook_name);
        
        // Create runbook sources from the fixture content
        let location = FileLocation::from_path(self.project_path.join("runbooks").join(&self.runbook_name));
        let mut sources = RunbookSources::new();
        sources.add_source(self.runbook_name.clone(), location.clone(), self.runbook_content.clone());
        
        // Create runbook instance
        let runbook_id = RunbookId::new(None, None, &self.runbook_name);
        let mut runbook = Runbook::new(runbook_id, None);
        
        // Create inputs map with our test inputs
        // Use from_environment_map to properly initialize with a default environment
        let mut env_map = IndexMap::new();
        let mut test_env = IndexMap::new();
        
        // Add all inputs to the test environment
        for (key, value) in &self.inputs {
            test_env.insert(key.clone(), value.clone());
        }
        
        env_map.insert("test".to_string(), test_env);
        let inputs_map = RunbookTopLevelInputsMap::from_environment_map(
            &Some("test".to_string()),
            &env_map
        );
        
        // Create contexts
        let auth_context = AuthorizationContext::new(location);
        let cloud_context = CloudServiceContext::empty();
        
        // Build contexts with addons
        runbook.build_contexts_from_sources(
            sources,
            inputs_map,
            auth_context,
            get_test_addon_by_namespace,
            cloud_context,
        ).await.map_err(|diagnostics| {
            // Fallback: create error from diagnostic messages
            let error_messages: Vec<String> = diagnostics.iter()
                .map(|d| d.message.clone())
                .collect();
            
            Report::new(EvmError::Config(
                ConfigError::ParseError(error_messages.join("; "))
            ))
        })?;
        
        // Execute unsupervised
        println!("Starting unsupervised execution...");
        let (tx, _rx) = txtx_addon_kit::channel::unbounded();
        println!("Created channel for unsupervised execution");
        let result = start_unsupervised_runbook_runloop(&mut runbook, &tx).await;
        println!("Unsupervised execution completed with result: {:?}", result.is_ok());
        
        match result {
            Ok(_final_state) => {
                println!("Runbook execution succeeded");
                
                // Return success - outputs will be read from state file
                Ok(TestResult {
                    success: true,
                    outputs: HashMap::new(), // Will be populated from state file
                    error: None,
                })
                

            }
            Err(diagnostics) => {
                // Fallback: create error from diagnostic messages
                let error_messages: Vec<String> = diagnostics.iter()
                    .map(|d| d.message.clone())
                    .collect();
                
                Err(Report::new(EvmError::Config(
                    ConfigError::ParseError(error_messages.join("; "))
                )))
            }
        }
    }
    
    /// Clean up the test project
    pub fn cleanup(&mut self) {
        if self.test_failed.load(Ordering::Relaxed) {
            // Preserve directory for debugging
            let preserved = self.temp_dir.path().to_path_buf();
            println!("Test failed - preserving directory at: {}", preserved.display());
            // Prevent TempDir from being dropped (and thus cleaned up)
            let _ = std::mem::ManuallyDrop::new(&self.temp_dir);
        }
        // Otherwise TempDir will clean up automatically on drop
    }
    
    /// Mark test as failed (preserves temp dir for debugging)
    pub fn mark_failed(&self) {
        self.test_failed.store(true, Ordering::Relaxed);
    }
}

/// Result from executing a runbook
#[derive(Debug)]
pub struct TestResult {
    pub success: bool,
    pub outputs: HashMap<String, Value>,
    pub error: Option<Report<EvmError>>,
}

impl ProjectTestHarness {
    /// Get an output value by name
    pub fn get_output(&self, name: &str) -> Option<Value> {
        // Find the most recent output JSON file in runs/testing/
        let runs_dir = self.project_path.join("runs/testing");
        if runs_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&runs_dir) {
                let mut output_files: Vec<_> = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.path().extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "json")
                            .unwrap_or(false)
                    })
                    .collect();
                
                // Sort by modification time to get the most recent
                output_files.sort_by_key(|entry| {
                    entry.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });
                
                if let Some(latest_file) = output_files.last() {
                    if let Ok(content) = std::fs::read_to_string(latest_file.path()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(value) = json.get(name) {
                                // Handle nested value structure {"value": ...}
                                if let Some(inner_value) = value.get("value") {
                                    return Some(json_to_txtx_value(inner_value));
                                } else {
                                    return Some(json_to_txtx_value(value));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Get a value from the test log object at a specific path
    /// Example: harness.get_log_path("action_logs.send_eth.tx_hash")
    pub fn get_log_path(&self, path: &str) -> Option<Value> {
        // First try to get the test_log output
        if let Some(log_output) = self.get_output("test_log") {
            return log_output.get_path(path).cloned();
        }
        None
    }
    
    /// Compare a log path with an expected value
    pub fn assert_log_path(&self, path: &str, expected: Value, message: &str) {
        let actual = self.get_log_path(path)
            .unwrap_or_else(|| panic!("Path '{}' not found in test log", path));
        
        let result = actual.compare_with(&expected);
        result.assert_matches(message);
    }
    
    /// Assert that a log object matches expected fields
    pub fn assert_log_object(&self, path: &str, expected: ExpectedValueBuilder) {
        let actual = self.get_log_path(path)
            .unwrap_or_else(|| panic!("Path '{}' not found in test log", path));
        
        let expected_value = expected.build();
        let result = actual.compare_with(&expected_value);
        result.assert_matches(&format!("Object at path '{}' doesn't match expected", path));
    }
    
    /// Check if an action was marked as successful in the log
    pub fn action_succeeded(&self, action_name: &str) -> bool {
        self.get_log_path(&format!("actions.{}.success", action_name))
            .and_then(|v| match v {
                Value::Bool(b) => Some(b),
                _ => None
            })
            .unwrap_or(false)
    }
    
    /// Get all logged data for an action
    pub fn get_action_log(&self, action_name: &str) -> Option<Value> {
        self.get_log_path(&format!("actions.{}", action_name))
    }
}

impl Drop for ProjectTestHarness {
    fn drop(&mut self) {
        // Cleanup will happen automatically unless marked as failed
        if self.test_failed.load(Ordering::Relaxed) {
            println!("Preserving failed test directory: {}", self.project_path.display());
        }
    }
}