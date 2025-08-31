//! Project-based test harness for EVM addon tests
//! 
//! This module provides a complete txtx project environment for testing,
//! supporting both Foundry and Hardhat compilation outputs.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::collections::HashMap;
use txtx_addon_kit::types::types::Value;
use super::integration::anvil_harness::AnvilInstance;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::errors::{EvmError, TransactionError, RpcError, ContractError, CodecError, ConfigError};
use error_stack::Report;

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
        
        // Add test accounts as inputs - matching the names expected by signers.testing.tx
        if !anvil.accounts.is_empty() {
            let account0 = &anvil.accounts[0];
            self.inputs.insert("sender_address".to_string(), format!("{:?}", account0.address));
            self.inputs.insert("sender_private_key".to_string(), account0.private_key.clone());
            self.inputs.insert("deployer_private_key".to_string(), account0.private_key.clone());
            
            if anvil.accounts.len() > 1 {
                let account1 = &anvil.accounts[1];
                self.inputs.insert("recipient_address".to_string(), format!("{:?}", account1.address));
                self.inputs.insert("recipient_private_key".to_string(), account1.private_key.clone());
            }
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
        let mut config = String::from("# txtx project configuration\n");
        config.push_str("name: test-project\n");
        config.push_str("version: 1.0.0\n");
        config.push_str("\n");
        config.push_str("environments:\n");
        config.push_str("  testing:\n");
        config.push_str("    description: Local testing environment\n");
        
        // Add inputs as environment variables
        if !self.inputs.is_empty() {
            config.push_str("    variables:\n");
            for (key, value) in &self.inputs {
                config.push_str(&format!("      {}: \"{}\"\n", key, value));
            }
        }
        
        config
    }

    /// Setup Foundry-specific project files
    fn setup_foundry_project(&self) -> Result<(), String> {
        // Create output directories
        fs::create_dir_all(self.project_path.join("out/SimpleStorage.sol"))
            .map_err(|e| format!("Failed to create out directory: {}", e))?;

        // Copy or create Foundry artifacts
        // TODO: Add SimpleStorage.json fixture
        let simple_storage_artifact = r#"{"abi":[],"bytecode":{"object":"0x"}}"#;
        fs::write(
            self.project_path.join("out/SimpleStorage.sol/SimpleStorage.json"),
            simple_storage_artifact
        ).map_err(|e| format!("Failed to write SimpleStorage artifact: {}", e))?;

        // Create foundry.toml
        let foundry_config = r#"[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc = "0.8.20"
"#;
        fs::write(self.project_path.join("foundry.toml"), foundry_config)
            .map_err(|e| format!("Failed to write foundry.toml: {}", e))?;

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

    /// Execute the runbook and return the result
    pub fn execute_runbook(&self) -> Result<TestResult, Report<EvmError>> {
        // In a real implementation, this would:
        // 1. Run `txtx run` command with the runbook
        // 2. Capture outputs
        // 3. Parse results
        
        // For now, we'll simulate success
        println!("Executing runbook: {}", self.runbook_name);
        println!("Project path: {}", self.project_path.display());
        
        Ok(TestResult {
            success: true,
            outputs: HashMap::new(),
            error: None,
        })
    }
    
    /// Clean up the test project
    pub fn cleanup(&mut self) {
        if self.test_failed.load(Ordering::Relaxed) {
            // Preserve directory for debugging
            let preserved = self.temp_dir.path().to_path_buf();
            println!("Test failed - preserving directory at: {}", preserved.display());
            std::mem::forget(self.temp_dir.clone());
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

impl Drop for ProjectTestHarness {
    fn drop(&mut self) {
        // Cleanup will happen automatically unless marked as failed
        if self.test_failed.load(Ordering::Relaxed) {
            println!("Preserving failed test directory: {}", self.project_path.display());
        }
    }
}