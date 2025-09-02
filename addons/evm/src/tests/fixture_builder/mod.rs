// Fixture builder system for EVM testing
// Provides template-based test fixtures with automatic output generation

pub mod accounts;
pub mod anvil_singleton;
pub mod anvil_manager;
pub mod runbook_parser;
pub mod executor;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_anvil;

#[cfg(test)]
mod example_test;
mod integration_test;
mod execution_test;
mod contract_test;
mod showcase_test;
mod test_cleanup;
pub mod helpers;
pub mod cleanup;

pub use accounts::NamedAccounts;
// Use manager that's backed by singleton
pub use anvil_manager::{AnvilManager, AnvilHandle, get_anvil_manager};
pub use anvil_singleton::cleanup_singleton;
pub use runbook_parser::RunbookParser;
pub use cleanup::{cleanup_test_infrastructure, force_cleanup_test_anvil};

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test fixture configuration
#[derive(Debug, Clone)]
pub struct FixtureConfig {
    pub test_name: String,
    pub template: Option<String>,
    pub environment: String,
    pub confirmations: u32,
    pub preserve_on_failure: bool,
    pub parameters: HashMap<String, String>,
}

impl Default for FixtureConfig {
    fn default() -> Self {
        Self {
            test_name: "test".to_string(),
            template: None,
            environment: "testing".to_string(),
            confirmations: 0,
            preserve_on_failure: true,
            parameters: HashMap::new(),
        }
    }
}

/// Builder for creating test fixtures
pub struct FixtureBuilder {
    config: FixtureConfig,
    anvil_manager: Option<Arc<Mutex<AnvilManager>>>,
    additional_contracts: Vec<(String, String)>,  // (name, source)
    additional_runbooks: Vec<(String, String)>,   // (name, content)
}

impl FixtureBuilder {
    /// Create a new fixture builder
    pub fn new(test_name: &str) -> Self {
        Self {
            config: FixtureConfig {
                test_name: test_name.to_string(),
                ..Default::default()
            },
            anvil_manager: None,
            additional_contracts: Vec::new(),
            additional_runbooks: Vec::new(),
        }
    }
    
    /// Use a template
    pub fn with_template(mut self, template: &str) -> Self {
        self.config.template = Some(template.to_string());
        self
    }
    
    /// Set the environment name
    pub fn with_environment(mut self, env: &str) -> Self {
        self.config.environment = env.to_string();
        self
    }
    
    /// Set default confirmations
    pub fn with_confirmations(mut self, confirmations: u32) -> Self {
        self.config.confirmations = confirmations;
        self
    }
    
    /// Add a parameter for template substitution
    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.config.parameters.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Add a contract to the project
    pub fn with_contract(mut self, name: &str, source: &str) -> Self {
        self.additional_contracts.push((name.to_string(), source.to_string()));
        self
    }
    
    /// Add a runbook to the project
    pub fn with_runbook(mut self, name: &str, content: &str) -> Self {
        self.additional_runbooks.push((name.to_string(), content.to_string()));
        self
    }
    
    /// Use the global Anvil manager
    pub fn with_anvil_manager(mut self, manager: Arc<Mutex<AnvilManager>>) -> Self {
        self.anvil_manager = Some(manager);
        self
    }
    
    /// Build the test fixture
    pub async fn build(self) -> Result<TestFixture, Box<dyn std::error::Error>> {
        eprintln!("🔨 Building test fixture: {}", self.config.test_name);
        
        // Get or create Anvil manager
        let anvil_manager = if let Some(manager) = self.anvil_manager {
            manager
        } else {
            get_anvil_manager().await?
        };
        
        // Get handle for this test
        let mut manager = anvil_manager.lock().await;
        let anvil_handle = manager.get_handle(&self.config.test_name).await?;
        drop(manager); // Release lock
        
        // Create temp directory for the project
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().to_path_buf();
        
        eprintln!("📁 Test directory: {}", project_dir.display());
        
        // Create project structure
        Self::create_project_structure(&project_dir)?;
        
        // Load and process template if specified
        if let Some(template) = &self.config.template {
            Self::apply_template(&project_dir, template, &self.config.parameters)?;
        }
        
        // Add additional contracts
        for (name, source) in &self.additional_contracts {
            Self::add_contract(&project_dir, name, source)?;
        }
        
        // Add additional runbooks
        for (name, content) in &self.additional_runbooks {
            Self::add_runbook(&project_dir, name, content)?;
        }
        
        // Generate txtx.yml with all runbook names
        let runbook_names: Vec<String> = self.additional_runbooks.iter()
            .map(|(name, _)| name.clone())
            .collect();
        let txtx_yml = Self::generate_txtx_yml(&self.config, &anvil_handle, &runbook_names);
        fs::write(project_dir.join("txtx.yml"), txtx_yml)?;
        
        // Create fixture
        Ok(TestFixture {
            temp_dir: Some(temp_dir),
            project_dir,
            config: self.config,
            anvil_manager,
            rpc_url: anvil_handle.url.clone(),
            anvil_handle,
            output_cache: HashMap::new(),
            output_files: Vec::new(),
        })
    }
    
    /// Create basic project structure
    fn create_project_structure(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("📁 Creating project structure in: {}", project_dir.display());
        fs::create_dir_all(project_dir.join("src"))?;
        fs::create_dir_all(project_dir.join("runbooks"))?;
        fs::create_dir_all(project_dir.join("runs/testing"))?;
        eprintln!("  ✅ Created directories: src/, runbooks/, runs/testing/");
        Ok(())
    }
    
    /// Apply a template to the project
    fn apply_template(
        project_dir: &Path,
        template: &str,
        parameters: &HashMap<String, String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement template loading and substitution
        eprintln!("📋 Applying template: {}", template);
        Ok(())
    }
    
    /// Add a contract to the project
    fn add_contract(
        project_dir: &Path,
        name: &str,
        source: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let contract_path = project_dir.join("src").join(format!("{}.sol", name));
        fs::write(contract_path, source)?;
        eprintln!("📝 Added contract: {}", name);
        Ok(())
    }
    
    /// Add a runbook to the project
    /// Creates a directory for the runbook with main.tx inside
    fn add_runbook(
        project_dir: &Path,
        name: &str,
        content: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse and inject outputs
        let parser = RunbookParser::new(content.to_string());
        let content_with_outputs = parser.inject_outputs();
        
        // Create runbook directory
        let runbook_dir = project_dir.join("runbooks").join(name);
        fs::create_dir_all(&runbook_dir)?;
        eprintln!("📁 Created runbook directory: {}", runbook_dir.display());
        
        // Write main.tx in the runbook directory
        let main_path = runbook_dir.join("main.tx");
        fs::write(&main_path, content_with_outputs)?;
        eprintln!("📝 Added runbook: {} at {}", name, main_path.display());
        eprintln!("  ✅ Auto-generated outputs injected");
        
        Ok(())
    }
    
    /// Generate txtx.yml configuration
    fn generate_txtx_yml(config: &FixtureConfig, anvil: &AnvilHandle, runbooks: &[String]) -> String {
        let accounts = anvil.accounts();
        
        // Build runbook entries - each points to a directory
        let runbook_entries = if runbooks.is_empty() {
            // Default main runbook
            eprintln!("⚠️  No runbooks specified, adding default 'main' runbook");
            format!("  - name: main\n    location: runbooks/main")
        } else {
            eprintln!("📝 Registering {} runbook(s) in txtx.yml", runbooks.len());
            runbooks.iter()
                .map(|name| {
                    eprintln!("  - Runbook: {} -> runbooks/{}/", name, name);
                    format!("  - name: {}\n    location: runbooks/{}", name, name)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        
        let yml_content = format!(r#"---
name: {}
id: {}
runbooks:
{}
environments:
  {}:
    confirmations: {}
    evm_chain_id: 31337
    evm_rpc_api_url: {}
    # Test accounts
    alice_address: "{}"
    alice_secret: "{}"
    bob_address: "{}"
    bob_secret: "{}"
    # Add more accounts as needed
"#,
            config.test_name,
            config.test_name,
            runbook_entries,
            config.environment,
            config.confirmations,
            anvil.url(),
            accounts.alice.address_string(),
            accounts.alice.secret_string(),
            accounts.bob.address_string(),
            accounts.bob.secret_string(),
        );
        
        eprintln!("📄 Generated txtx.yml with {} environment", config.environment);
        yml_content
    }
}

/// Active test fixture
pub struct TestFixture {
    temp_dir: Option<TempDir>,
    pub project_dir: PathBuf,
    pub config: FixtureConfig,
    anvil_manager: Arc<Mutex<AnvilManager>>,
    pub anvil_handle: AnvilHandle,
    pub rpc_url: String,
    pub output_cache: HashMap<String, HashMap<String, txtx_addon_kit::types::types::Value>>,
    pub output_files: Vec<PathBuf>,
}

impl TestFixture {
    /// Execute a runbook
    pub async fn execute_runbook(&mut self, runbook_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("\n🎯 TestFixture::execute_runbook({})", runbook_name);
        eprintln!("  Project: {}", self.project_dir.display());
        
        // Verify runbook exists
        let runbook_dir = self.project_dir.join("runbooks").join(runbook_name);
        if !runbook_dir.exists() {
            eprintln!("  ❌ ERROR: Runbook directory doesn't exist: {}", runbook_dir.display());
            eprintln!("  📁 Available runbook directories:");
            if let Ok(entries) = fs::read_dir(self.project_dir.join("runbooks")) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        eprintln!("    - {}", entry.file_name().to_string_lossy());
                    }
                }
            }
            return Err(format!("Runbook directory not found: {}", runbook_dir.display()).into());
        }
        
        // Prepare inputs including account information
        let mut inputs = HashMap::new();
        
        // Add RPC URL and chain ID
        inputs.insert("rpc_url".to_string(), self.anvil_handle.url().to_string());
        inputs.insert("chain_id".to_string(), "31337".to_string());
        
        // Add account addresses and secrets
        let accounts = self.anvil_handle.accounts();
        for (key, value) in accounts.as_inputs() {
            inputs.insert(key, value);
        }
        
        // Add any custom parameters
        for (key, value) in &self.config.parameters {
            inputs.insert(key.clone(), value.clone());
        }
        
        eprintln!("  📊 Total inputs: {} parameters", inputs.len());
        
        // Execute via CLI
        let result = executor::execute_runbook(
            &self.project_dir,
            runbook_name,
            &self.config.environment,
            &inputs,
        )?;
        
        if !result.success {
            eprintln!("  ❌ Runbook execution failed!");
            eprintln!("    Stderr: {}", result.stderr);
            if !result.stdout.is_empty() {
                eprintln!("    Stdout: {}", result.stdout);
            }
            return Err(format!("Runbook execution failed: {}", result.stderr).into());
        }
        
        eprintln!("  ✅ Runbook executed successfully");
        eprintln!("  📊 Outputs captured: {} values", result.outputs.len());
        
        // Cache the outputs
        self.output_cache.insert(runbook_name.to_string(), result.outputs);
        self.output_files.push(result.output_file);
        
        Ok(())
    }
    
    /// Execute with specific confirmations
    pub async fn execute_with_confirmations(
        &mut self,
        runbook_name: &str,
        confirmations: u32
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Execute runbook
        self.execute_runbook(runbook_name).await?;
        
        // Mine blocks if needed
        if confirmations > 0 {
            let manager = self.anvil_manager.lock().await;
            manager.mine_blocks(confirmations).await?;
        }
        
        Ok(())
    }
    
    /// Get outputs for a runbook
    pub fn get_outputs(&self, runbook_name: &str) -> Option<&HashMap<String, txtx_addon_kit::types::types::Value>> {
        self.output_cache.get(runbook_name)
    }
    
    /// Get a specific output value
    pub fn get_output(&self, runbook_name: &str, output_name: &str) -> Option<&txtx_addon_kit::types::types::Value> {
        self.output_cache.get(runbook_name)?.get(output_name)
    }
    
    /// Add a runbook to an existing fixture
    pub fn add_runbook(&mut self, name: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        FixtureBuilder::add_runbook(&self.project_dir, name, content)
    }
    
    /// Add a contract to an existing fixture
    pub fn add_contract(&mut self, name: &str, source: &str) -> Result<(), Box<dyn std::error::Error>> {
        FixtureBuilder::add_contract(&self.project_dir, name, source)
    }
    
    /// Take a checkpoint (snapshot)
    pub async fn checkpoint(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let mut manager = self.anvil_manager.lock().await;
        let checkpoint_id = format!("checkpoint_{}", self.output_files.len());
        manager.snapshot(&checkpoint_id).await
    }
    
    /// Revert to a checkpoint
    pub async fn revert(&mut self, checkpoint_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = self.anvil_manager.lock().await;
        manager.revert(checkpoint_id).await
    }
    
    /// Restore from a checkpoint
    pub async fn restore(&mut self, snapshot_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = self.anvil_manager.lock().await;
        manager.revert(snapshot_id).await?;
        Ok(())
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        if self.config.preserve_on_failure {
            eprintln!("📁 Preserving test directory: {}", self.project_dir.display());
            // Prevent temp_dir from cleaning up
            self.temp_dir.take();
        }
    }
}