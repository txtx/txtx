use crate::builders::runbook_builder::{RunbookBuilder, ValidationResult};
use std::collections::HashMap;
use std::path::PathBuf;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_core::manifest::WorkspaceManifest;

/// Enhanced validation options for RunbookBuilder
pub enum ValidationMode {
    /// Basic HCL validation only (default)
    HclOnly,
    /// Full linter validation with manifest and environment context
    Linter {
        /// Optional manifest for input/environment validation
        manifest: Option<WorkspaceManifest>,
        /// Optional environment name to use
        environment: Option<String>,
        /// Optional file path for error reporting
        file_path: Option<PathBuf>,
    },
    /// LSP validation with workspace context
    Lsp {
        /// Workspace root for multi-file resolution
        workspace_root: PathBuf,
        /// Optional manifest for context
        manifest: Option<WorkspaceManifest>,
    },
}

/// Extension trait for RunbookBuilder to enable linter validation
///
/// This trait must be implemented by the test crate that has access to txtx-cli.
/// This avoids a circular dependency between txtx-test-utils and txtx-cli.
///
/// # Example Implementation
///
/// ```rust,ignore
/// use txtx_test_utils::{RunbookBuilder, RunbookBuilderExt, ValidationResult};
/// use txtx_cli::cli::linter_impl::analyzer::RunbookAnalyzer;
///
/// impl RunbookBuilderExt for RunbookBuilder {
///     fn validate_with_linter_impl(
///         &mut self,
///         content: &str,
///         manifest: Option<&WorkspaceManifest>,
///         environment: Option<&String>,
///         cli_inputs: &[(String, String)],
///         file_path: &Path,
///     ) -> ValidationResult {
///         let analyzer = RunbookAnalyzer::new();
///         let core_result = analyzer.analyze_runbook_with_context(
///             file_path,
///             content,
///             manifest,
///             environment,
///             cli_inputs,
///         );
///         
///         // Convert core ValidationResult to test utils ValidationResult
///         ValidationResult {
///             success: core_result.errors.is_empty(),
///             errors: /* convert errors */,
///             warnings: /* convert warnings */,
///         }
///     }
/// }
/// ```
pub trait RunbookBuilderExt {
    /// Implementation hook for linter validation
    fn validate_with_linter_impl(
        &mut self,
        content: &str,
        manifest: Option<&WorkspaceManifest>,
        environment: Option<&String>,
        cli_inputs: &[(String, String)],
        file_path: &std::path::Path,
    ) -> ValidationResult;
}

impl RunbookBuilder {
    /// Validate with enhanced linter analysis
    ///
    /// This runs the full linter validation pipeline including:
    /// - Undefined signer detection
    /// - Invalid field access on action outputs
    /// - Cross-reference validation between actions
    /// - Input/environment variable validation against manifest
    ///
    /// Note: This method requires the RunbookBuilderExt trait to be implemented
    /// in your test crate with access to txtx-cli.
    ///
    /// # Example
    /// ```rust,ignore
    /// use txtx_test_utils::{RunbookBuilder, assert_validation_error};
    /// use some_helper::create_test_manifest;
    /// 
    /// let manifest = create_test_manifest();
    /// let result = RunbookBuilder::new()
    ///     .action("deploy", "evm::deploy_contract")
    ///         .input("signer", "signer.undefined")  // Linter will catch this!
    ///     .validate_with_linter(Some(manifest), Some("production".to_string()));
    ///
    /// assert_validation_error!(result, "undefined signer");
    /// ```
    pub fn validate_with_linter(
        &mut self,
        manifest: Option<WorkspaceManifest>,
        environment: Option<String>,
    ) -> ValidationResult {
        self.validate_with_mode(ValidationMode::Linter {
            manifest,
            environment,
            file_path: Some(PathBuf::from("test.tx")),
        })
    }

    /// Validate with specific validation mode
    pub fn validate_with_mode(&mut self, mode: ValidationMode) -> ValidationResult {
        let content = self.build_content();

        match mode {
            ValidationMode::HclOnly => {
                // Use existing simple validation
                crate::simple_validator::validate_content(&content)
            }
            ValidationMode::Linter { manifest, environment, file_path } => {
                // Use the same HCL validator as the actual linter command
                use crate::addon_registry::{extract_addon_specifications, get_all_addons};
                use txtx_addon_kit::types::diagnostics::Diagnostic;
                use txtx_core::validation::{
                    hcl_validator, manifest_validator::validate_inputs_against_manifest,
                    ValidationResult as CoreResult,
                };

                // Create core validation result
                let mut core_result = CoreResult {
                    errors: Vec::new(),
                    warnings: Vec::new(),
                    suggestions: Vec::new(),
                };

                // Get addon specifications
                let addons = get_all_addons();
                let addon_specs = extract_addon_specifications(&addons);

                // Determine file path
                let file_path_str = file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "test.tx".to_string());

                // Run HCL validation with addon specifications
                match hcl_validator::validate_with_hcl_and_addons(
                    &content,
                    &mut core_result,
                    &file_path_str,
                    addon_specs,
                ) {
                    Ok(input_refs) => {
                        // If we have manifest context, validate inputs
                        if let (Some(manifest), Some(env_name)) = (&manifest, &environment) {
                            // Convert CLI inputs from builder
                            let cli_inputs: Vec<(String, String)> = vec![];

                            validate_inputs_against_manifest(
                                &input_refs,
                                &content,
                                manifest,
                                Some(env_name),
                                &mut core_result,
                                &file_path_str,
                                &cli_inputs,
                                txtx_core::validation::manifest_validator::ManifestValidationConfig::default(),
                            );
                        }
                    }
                    Err(e) => {
                        core_result.errors.push(
                            txtx_core::validation::Diagnostic::error(
                                format!("Failed to parse runbook: {}", e)
                            ).with_file(file_path_str.clone())
                        );
                    }
                }

                // Convert core result to our result type
                let errors: Vec<Diagnostic> = core_result
                    .errors
                    .into_iter()
                    .map(|e| Diagnostic::error_from_string(e.message))
                    .collect();

                let warnings: Vec<Diagnostic> = core_result
                    .warnings
                    .into_iter()
                    .map(|w| Diagnostic::warning_from_string(w.message))
                    .collect();

                ValidationResult { success: errors.is_empty(), errors, warnings }
            }
            ValidationMode::Lsp { workspace_root: _, manifest: _ } => {
                // LSP validation requires the RunbookBuilderExt trait to be implemented
                // by the test crate that has access to txtx-cli
                // For now, we provide a simple fallback that uses HCL validation
                eprintln!("INFO: Using basic HCL validation for LSP mode. Implement RunbookBuilderExt::validate_with_lsp_impl for full LSP validation.");

                // Use HCL validation as a fallback
                crate::simple_validator::validate_content(&content)
            }
        }
    }

    /// Create a test manifest with the configured environments
    pub fn build_manifest(&self) -> WorkspaceManifest {
        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: Vec::new(),
            environments: IndexMap::new(),
            location: None,
        };

        // Add configured environments to manifest
        for (env_name, vars) in &self.environments {
            let env_vars: IndexMap<String, String> =
                vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            manifest.environments.insert(env_name.clone(), env_vars);
        }

        manifest
    }
}

/// Helper to create a test manifest quickly
pub fn create_test_manifest_with_env(
    environments: Vec<(&str, Vec<(&str, &str)>)>,
) -> WorkspaceManifest {
    let mut manifest = WorkspaceManifest {
        name: "test".to_string(),
        id: "test-id".to_string(),
        runbooks: Vec::new(),
        environments: IndexMap::new(),
        location: None,
    };

    for (env_name, vars) in environments {
        let mut env_map = IndexMap::new();
        for (key, value) in vars {
            env_map.insert(key.to_string(), value.to_string());
        }
        manifest.environments.insert(env_name.to_string(), env_map);
    }

    manifest
}

/// Create a test manifest from a HashMap of environments
pub fn create_test_manifest_from_envs(
    environments: &HashMap<String, HashMap<String, String>>,
) -> WorkspaceManifest {
    let mut manifest = WorkspaceManifest {
        name: "test".to_string(),
        id: "test-id".to_string(),
        runbooks: Vec::new(),
        environments: IndexMap::new(),
        location: None,
    };

    for (env_name, vars) in environments {
        let mut env_map = IndexMap::new();
        for (key, value) in vars {
            env_map.insert(key.clone(), value.clone());
        }
        manifest.environments.insert(env_name.clone(), env_map);
    }

    manifest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_validation_error;

    #[test]
    fn test_linter_catches_undefined_signer() {
        // This test would fail with HCL-only validation but passes with linter
        let result = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("deploy", "evm::deploy_contract")
            .input("signer", "signer.undefined_signer")
            .validate_with_linter(None, None);

        // Linter validation catches undefined signers!
        assert_validation_error!(result, "undefined_signer");
    }

    // TODO: These tests require more advanced linter validation
    // #[test]
    // fn test_linter_validates_action_outputs() {
    //     // Test that linter catches invalid field access
    //     let result = RunbookBuilder::new()
    //         .addon("evm", vec![])
    //         .action("send", "evm::send_eth")
    //             .input("to", "0x123")
    //             .input("value", "1000")
    //         .output("bad", "action.send.invalid_field")  // send_eth only has tx_hash
    //         .validate_with_linter(None, None);

    //     assert_validation_error!(result, "Field 'invalid_field' does not exist");
    // }

    // #[test]
    // fn test_linter_validates_inputs_against_manifest() {
    //     // Create a manifest with environment variables
    //     let manifest = create_test_manifest_with_env(vec![
    //         ("production", vec![("API_URL", "https://api.example.com")]),
    //     ]);

    //     // Test missing input validation
    //     let result = RunbookBuilder::new()
    //         .variable("key", "env.MISSING_KEY")
    //         .output("result", "input.key")
    //         .validate_with_linter(Some(manifest), Some("production".to_string()));

    //     assert_validation_error!(result, "MISSING_KEY");
    // }

    #[test]
    fn test_hcl_vs_linter_validation() {
        // Test case 1: Valid runbook that passes HCL validation  
        let mut runbook_valid = RunbookBuilder::new()
            .addon("evm", vec![("chain_id", "1")])
            .signer("deployer", "evm::web_wallet", vec![])
            .action("send", "evm::send_eth")
            .input("signer", "signer.deployer")
            .input("recipient_address", "0x1234567890123456789012345678901234567890")
            .input("amount", "1000000000000000000"); // 1 ETH in wei

        // HCL validation should pass for valid runbook
        let hcl_result = runbook_valid.validate();
        
        // Debug: Print errors if validation fails
        if !hcl_result.success {
            println!("HCL validation errors:");
            for error in &hcl_result.errors {
                println!("  - {}", error.message);
            }
        }
        
        assert!(hcl_result.success, "HCL validation should pass for valid runbook");

        // Linter validation should also pass
        let linter_result = runbook_valid.validate_with_linter(None, None);
        assert!(linter_result.success, "Linter validation should pass for valid runbook");
        
        // Test case 2: Runbook with undefined signer - linter catches this
        let mut runbook_with_undefined = RunbookBuilder::new()
            .addon("evm", vec![("chain_id", "1")])
            // Note: no signer defined
            .action("send", "evm::send_eth")
            .input("signer", "signer.undefined") // This signer doesn't exist
            .input("recipient_address", "0x1234567890123456789012345678901234567890")
            .input("amount", "1000000000000000000");

        // HCL validation might pass (depends on implementation)
        let _hcl_result2 = runbook_with_undefined.validate();
        
        // Linter validation should fail for undefined signer
        let linter_result2 = runbook_with_undefined.validate_with_linter(None, None);
        assert!(!linter_result2.success, "Linter should catch undefined signer");
        assert!(
            linter_result2.errors.iter().any(|e| e.message.contains("undefined")),
            "Error should mention undefined signer"
        );
    }

    #[test]
    fn test_env_var_validation() {
        let manifest = create_test_manifest_with_env(vec![
            ("development", vec![("API_KEY", "test-key")]),
            ("production", vec![("API_KEY", "prod-key"), ("DB_URL", "postgres://...")]),
        ]);

        // Test missing env var
        let result = RunbookBuilder::new()
            .variable("key", "env.MISSING_KEY")
            .output("result", "variable.key")
            .validate_with_linter(Some(manifest.clone()), Some("production".to_string()));

        assert_validation_error!(result, "MISSING_KEY");

        // Test valid env var
        let result2 = RunbookBuilder::new()
            .variable("key", "env.API_KEY")
            .output("result", "variable.key")
            .validate_with_linter(Some(manifest), Some("production".to_string()));

        assert!(result2.success);
    }
}
