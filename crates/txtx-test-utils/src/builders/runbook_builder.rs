use std::collections::HashMap;
use txtx_addon_kit::serde_json;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_core::manifest::WorkspaceManifest;

/// Validation result for a runbook
#[derive(Debug)]
pub struct ValidationResult {
    pub success: bool,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

/// Parse result for a runbook
#[derive(Debug)]
pub struct ParseResult {
    pub runbook: Option<String>,
    pub errors: Vec<String>,
}

/// Execution result for a runbook
pub struct ExecutionResult {
    pub success: bool,
    pub outputs: HashMap<String, String>,
    pub errors: Vec<String>,
}

/// Builder for creating and testing runbooks
///
/// # Overview
///
/// `RunbookBuilder` provides a fluent API for constructing test runbooks and validating them.
/// It simplifies test writing by offering a clean, chainable interface for building runbook
/// content programmatically.
///
/// # Capabilities
///
/// - **HCL Syntax Validation**: Validates runbook syntax using the HCL parser
/// - **Basic Semantic Validation**: Catches errors like unknown namespaces, invalid action types
/// - **Fluent API**: Chain methods to build complex runbooks easily
/// - **Environment Support**: Define environment variables for testing
/// - **CLI Input Support**: Simulate CLI input overrides
///
/// # Limitations
///
/// `RunbookBuilder` uses `txtx_core::validation::hcl_validator` which provides HCL parsing
/// and basic validation. It does **NOT** include the enhanced validation that the `doctor`
/// command provides:
///
/// - **No Signer Reference Validation**: Won't catch undefined signer references
/// - **No Action Output Validation**: Won't validate if action output fields exist
/// - **No Cross-Reference Validation**: Won't check if referenced actions are defined
/// - **No Flow Validation**: Won't validate flow variables or flow-specific rules
/// - **No Multi-File Support**: Cannot test multi-file runbook imports
/// - **No Input/Environment Validation**: Won't verify if inputs have corresponding env vars
///
/// # When to Use
///
/// Use `RunbookBuilder` for:
/// - Testing HCL syntax correctness
/// - Testing basic semantic errors (unknown namespaces, action types)
/// - Unit testing runbook construction logic
/// - Quick validation tests that don't need full doctor analysis
///
/// # When NOT to Use
///
/// Keep integration tests for:
/// - Testing doctor command's enhanced validation
/// - Testing specific error messages and line numbers
/// - Testing multi-file runbooks
/// - Testing flow validation
/// - Testing the full validation pipeline
///
/// # Example
///
/// ```rust
/// let result = RunbookBuilder::new()
///     .addon("evm", vec![("network_id", "1")])
///     .action("deploy", "evm::deploy_contract")
///         .input("contract", "MyContract")
///     .validate();
///
/// assert!(result.success);
/// ```
#[derive(Clone)]
pub struct RunbookBuilder {
    /// The main runbook content
    content: String,
    /// Additional files for multi-file runbooks
    files: HashMap<String, String>,
    /// Environment variables by environment name
    pub(crate) environments: HashMap<String, HashMap<String, String>>,
    /// Mock blockchain configurations
    mocks: HashMap<String, MockConfig>,
    /// CLI inputs
    pub(crate) cli_inputs: HashMap<String, String>,
    /// Current building state for fluent API
    building_content: Vec<String>,
    /// Current action being built
    current_action: Option<String>,
    /// Optional manifest for validation
    manifest: Option<WorkspaceManifest>,
    /// Current environment for validation
    current_environment: Option<String>,
}

/// Configuration for a mock blockchain
#[derive(Clone)]
pub struct MockConfig {
    pub chain_type: String,
    pub initial_state: serde_json::Value,
}

impl RunbookBuilder {
    /// Create a new runbook builder
    pub fn new() -> Self {
        Self {
            content: String::new(),
            files: HashMap::new(),
            environments: HashMap::new(),
            mocks: HashMap::new(),
            cli_inputs: HashMap::new(),
            building_content: Vec::new(),
            current_action: None,
            manifest: None,
            current_environment: None,
        }
    }

    /// Set the main runbook content
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    /// Add a file for multi-file runbooks
    pub fn with_file(mut self, path: &str, content: &str) -> Self {
        self.files.insert(path.to_string(), content.to_string());
        self
    }

    /// Add environment variables
    pub fn with_environment(mut self, env_name: &str, vars: Vec<(&str, &str)>) -> Self {
        let env_vars: HashMap<String, String> =
            vars.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        self.environments.insert(env_name.to_string(), env_vars);
        self
    }

    /// Add CLI input
    pub fn with_cli_input(mut self, key: &str, value: &str) -> Self {
        self.cli_inputs.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a mock blockchain
    pub fn with_mock(mut self, name: &str, config: MockConfig) -> Self {
        self.mocks.insert(name.to_string(), config);
        self
    }

    /// Add an addon
    pub fn addon(mut self, name: &str, config: Vec<(&str, &str)>) -> Self {
        let config_str = config
            .into_iter()
            .map(|(k, v)| format!("{} = {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        self.building_content.push(format!(r#"addon "{}" {{ {} }}"#, name, config_str));
        self
    }

    /// Add a variable
    pub fn variable(mut self, name: &str, value: &str) -> Self {
        self.building_content.push(format!(
            r#"
variable "{}" {{
    value = {}
}}"#,
            name,
            if value.starts_with("env.")
                || value.starts_with("input.")
                || value.starts_with("action.")
                || value.starts_with("variable.")
            {
                value.to_string()
            } else {
                format!(r#""{}""#, value)
            }
        ));
        self
    }

    /// Add an action
    pub fn action(mut self, name: &str, action_type: &str) -> Self {
        // Close any previous action
        if self.current_action.is_some() {
            self.building_content.push("}".to_string());
        }
        self.current_action = Some(name.to_string());
        self.building_content.push(format!(
            r#"
action "{}" "{}" {{"#,
            name, action_type
        ));
        self
    }

    /// Add an input to the current action
    pub fn input(mut self, name: &str, value: &str) -> Self {
        if self.current_action.is_some() {
            self.building_content.push(format!(
                "    {} = {}",
                name,
                if value.starts_with("signer.")
                    || value.starts_with("input.")
                    || value.starts_with("action.")
                    || value.starts_with("variable.")
                    || value.parse::<i64>().is_ok()
                {
                    value.to_string()
                } else {
                    format!(r#""{}""#, value)
                }
            ));
        }
        self
    }

    /// Add an output
    pub fn output(mut self, name: &str, value: &str) -> Self {
        // Close any open action
        if self.current_action.is_some() {
            self.building_content.push("}".to_string());
            self.current_action = None;
        }
        self.building_content.push(format!(
            r#"
output "{}" {{
    value = {}
}}"#,
            name, value
        ));
        self
    }

    /// Add a signer
    pub fn signer(mut self, name: &str, signer_type: &str, config: Vec<(&str, &str)>) -> Self {
        // Close any open action
        if self.current_action.is_some() {
            self.building_content.push("}".to_string());
            self.current_action = None;
        }

        let config_lines = config
            .into_iter()
            .map(|(k, v)| format!("    {} = \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        self.building_content.push(format!(
            r#"
signer "{}" "{}" {{
{}
}}"#,
            name, signer_type, config_lines
        ));
        self
    }

    /// Get the content being built (for internal use by extensions)
    #[allow(dead_code)]
    pub(crate) fn get_content(&self) -> &str {
        &self.content
    }

    // Helper for test harness (not used by builder itself)
    #[allow(dead_code)]
    pub(crate) fn get_files(&self) -> &HashMap<String, String> {
        &self.files
    }

    /// Build the final content
    pub fn build_content(&mut self) -> String {
        // Close any open action
        if self.current_action.is_some() {
            self.building_content.push("}".to_string());
            self.current_action = None;
        }

        if !self.content.is_empty() {
            self.content.clone()
        } else {
            self.building_content.join("\n")
        }
    }

    /// Parse the runbook without validation
    /// Set the workspace manifest for validation
    pub fn with_manifest(mut self, manifest: WorkspaceManifest) -> Self {
        self.manifest = Some(manifest);
        self
    }

    /// Set the current environment for validation
    pub fn set_current_environment(mut self, env: &str) -> Self {
        self.current_environment = Some(env.to_string());
        self
    }

    /// Validate with manifest checking enabled
    ///
    /// This method enables manifest validation with a specific environment.
    /// Without specifying an environment, validation can only check against "defaults",
    /// which may not include all variables needed for actual environments.
    ///
    /// For proper validation, always use set_current_environment() first:
    /// ```
    /// builder.set_current_environment("production").validate_with_manifest()
    /// ```
    pub fn validate_with_manifest(&mut self) -> ValidationResult {
        let content = self.build_content();
        let cli_inputs_vec: Vec<(String, String)> =
            self.cli_inputs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        let manifest = self
            .manifest
            .clone()
            .unwrap_or_else(|| crate::builders::create_test_manifest_from_envs(&self.environments));

        crate::simple_validator::validate_content_with_manifest(
            &content,
            Some(manifest),
            self.current_environment.clone(),
            cli_inputs_vec,
        )
    }

    pub fn parse(&self) -> ParseResult {
        // TODO: Implement actual parsing
        // For now, return a placeholder
        ParseResult { runbook: None, errors: vec![] }
    }

    /// Validate the runbook without execution
    pub fn validate(&mut self) -> ValidationResult {
        let content = self.build_content();

        // Convert CLI inputs to vector format
        let cli_inputs_vec: Vec<(String, String)> =
            self.cli_inputs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Only use manifest-aware validation if we have both a manifest/environments AND a current environment
        // Without specifying an environment, we can only validate against "defaults" which is incomplete
        if (self.manifest.is_some() || !self.environments.is_empty())
            && self.current_environment.is_some()
        {
            // Create a manifest if we don't have one but have environments
            let manifest = self.manifest.clone().unwrap_or_else(|| {
                crate::builders::create_test_manifest_from_envs(&self.environments)
            });

            crate::simple_validator::validate_content_with_manifest(
                &content,
                Some(manifest),
                self.current_environment.clone(),
                cli_inputs_vec,
            )
        } else {
            // Fall back to simple HCL validation
            // This is appropriate when:
            // - No manifest/environments are provided (pure syntax validation)
            // - Environments are provided but no current environment is set (can't validate properly)
            crate::simple_validator::validate_content(&content)
        }
    }

    /// Execute the runbook
    pub async fn execute(&self) -> ExecutionResult {
        // TODO: Implement actual execution
        // For now, return a placeholder
        ExecutionResult { success: true, outputs: HashMap::new(), errors: vec![] }
    }
}
