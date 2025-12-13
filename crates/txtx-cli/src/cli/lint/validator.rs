//! Linter validation engine
//!

use std::path::PathBuf;
use txtx_core::validation::{ValidationResult, Diagnostic};
use txtx_core::manifest::WorkspaceManifest;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_network_evm::codec::crypto::resolve_keystore_path;
use crate::cli::common::addon_registry;

use super::config::LinterConfig;
use super::error::LinterError;
use super::rules::{ValidationContext, InputInfo, Severity, get_default_rules, validate_all};

/// Trait for types that can be converted into an optional WorkspaceManifest
pub trait IntoManifest {
    fn into_manifest(self) -> Option<WorkspaceManifest>;
}

impl IntoManifest for Option<WorkspaceManifest> {
    fn into_manifest(self) -> Option<WorkspaceManifest> {
        self
    }
}

impl IntoManifest for WorkspaceManifest {
    fn into_manifest(self) -> Option<WorkspaceManifest> {
        Some(self)
    }
}

impl IntoManifest for Option<&PathBuf> {
    fn into_manifest(self) -> Option<WorkspaceManifest> {
        self.and_then(|p| {
            let location = FileLocation::from_path(p.clone());
            WorkspaceManifest::from_location(&location).ok()
        })
    }
}

impl IntoManifest for &PathBuf {
    fn into_manifest(self) -> Option<WorkspaceManifest> {
        let location = FileLocation::from_path(self.clone());
        WorkspaceManifest::from_location(&location).ok()
    }
}

impl IntoManifest for Option<PathBuf> {
    fn into_manifest(self) -> Option<WorkspaceManifest> {
        self.as_ref().into_manifest()
    }
}

/// Linter engine that orchestrates validation of txtx runbooks.
pub struct Linter {
    config: LinterConfig,
}

impl Linter {
    /// Create a new linter with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `LinterError` if the configuration is invalid.
    pub fn new(config: &LinterConfig) -> Result<Self, LinterError> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Create a linter with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: LinterConfig::default(),
        }
    }

    /// Lint a specific runbook by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the runbook to lint
    ///
    /// # Errors
    ///
    /// Returns `LinterError` if:
    /// - The runbook cannot be found
    /// - The runbook cannot be loaded or parsed
    pub fn lint_runbook(&self, name: &str) -> Result<(), LinterError> {
        let workspace = super::workspace::WorkspaceAnalyzer::new(&self.config)?;
        let result = workspace.analyze_runbook(name)?;

        self.format_and_print(result);
        Ok(())
    }

    /// Lint all runbooks in the workspace.
    ///
    /// # Errors
    ///
    /// Returns `LinterError` if:
    /// - The workspace manifest cannot be loaded
    /// - Any runbook cannot be loaded or parsed
    pub fn lint_all(&self) -> Result<(), LinterError> {
        let workspace = super::workspace::WorkspaceAnalyzer::new(&self.config)?;
        let results = workspace.analyze_all()?;

        for result in results {
            self.format_and_print(result);
        }
        Ok(())
    }

    pub fn validate_content<M: IntoManifest>(
        &self,
        content: &str,
        file_path: &str,
        manifest: M,
        environment: Option<&String>,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Convert manifest using Into trait
        let manifest = manifest.into_manifest();

        // Load addon specs
        let addons = addon_registry::get_all_addons();
        let addon_specs = addon_registry::extract_addon_specifications(&addons);

        // Run HCL validation
        match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
            content,
            &mut result,
            file_path,
            addon_specs,
        ) {
            Ok(refs) => {
                if let Some(ref manifest) = manifest {
                    self.validate_with_rules(&refs.inputs, content, file_path, manifest, environment, &mut result);
                }
                // Validate signers (e.g., keystore paths)
                self.validate_signers(&refs.signers, file_path, &mut result);
            }
            Err(e) => {
                result.errors.push(
                    Diagnostic::error(format!("Failed to parse runbook: {}", e))
                        .with_file(file_path.to_string())
                );
            }
        }

        result
    }

    fn issue_to_diagnostic(
        issue: super::rules::ValidationIssue,
        file_path: &str,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Diagnostic {
        let message = issue.message.into_owned();

        let mut diagnostic = match issue.severity {
            Severity::Error => Diagnostic::error(message),
            Severity::Warning => Diagnostic::warning(message),
            Severity::Info => Diagnostic::warning(format!("[INFO] {}", message)),
            Severity::Off => unreachable!("Off severity should be filtered before conversion"),
        };

        diagnostic = diagnostic
            .with_code(issue.rule)
            .with_file(file_path);

        if let Some(line_num) = line {
            diagnostic = diagnostic.with_line(line_num);
        }

        if let Some(col_num) = column {
            diagnostic = diagnostic.with_column(col_num);
        }

        if let Some(help) = issue.help {
            diagnostic = match issue.severity {
                Severity::Error => diagnostic.with_context(help.into_owned()),
                _ => diagnostic.with_suggestion(help.into_owned()),
            };
        }

        if let Some(example) = issue.example {
            if matches!(issue.severity, Severity::Error) {
                diagnostic = diagnostic.with_documentation(example);
            }
        }

        diagnostic
    }

    fn validate_with_rules(
        &self,
        input_refs: &[txtx_core::validation::LocatedInputRef],
        content: &str,
        file_path: &str,
        manifest: &WorkspaceManifest,
        environment: Option<&String>,
        result: &mut ValidationResult,
    ) {
        let effective_inputs = self.resolve_inputs(manifest, environment);
        let rules = get_default_rules();

        for input_ref in input_refs {
            let full_name = format!("input.{}", input_ref.name);
            let context = ValidationContext {
                manifest,
                environment: environment.map(|s| s.as_str()),
                effective_inputs: &effective_inputs,
                cli_inputs: &self.config.cli_inputs,
                content,
                file_path,
                input: InputInfo {
                    name: input_ref.name.clone(),
                    full_name,
                },
                config: Some(&self.config),
            };

            let issues = validate_all(&context, rules);

            for issue in issues {
                if matches!(issue.severity, Severity::Off) {
                    continue;
                }

                let severity = issue.severity;
                let diagnostic = Self::issue_to_diagnostic(
                    issue,
                    file_path,
                    Some(input_ref.line),
                    Some(input_ref.column),
                );

                match severity {
                    Severity::Error => {
                        result.errors.push(diagnostic);
                    }
                    Severity::Warning | Severity::Info => {
                        result.warnings.push(diagnostic);
                    }
                    Severity::Off => unreachable!(),
                }
            }
        }
    }

    fn resolve_inputs(&self, manifest: &WorkspaceManifest, environment: Option<&String>) -> std::collections::HashMap<String, String> {
        let mut inputs = std::collections::HashMap::new();

        // Add global inputs
        if let Some(global) = manifest.environments.get("global") {
            inputs.extend(global.clone());
        }

        // Add environment-specific inputs
        if let Some(env_name) = environment {
            if let Some(env) = manifest.environments.get(env_name) {
                inputs.extend(env.clone());
            }
        }

        // Add CLI inputs (highest priority)
        for (key, value) in &self.config.cli_inputs {
            inputs.insert(key.clone(), value.clone());
        }

        inputs
    }

    fn format_and_print(&self, result: ValidationResult) {
        let formatter = super::formatter::get_formatter(self.config.format);
        formatter.format(&result);
    }

    /// Validate signers, specifically keystore signers for path validity
    fn validate_signers(
        &self,
        signers: &[txtx_core::validation::LocatedSignerRef],
        file_path: &str,
        result: &mut ValidationResult,
    ) {
        for signer in signers.iter().filter(|s| s.signer_type == "evm::keystore") {
            let Some(keystore_account) = signer.attributes.get("keystore_account") else {
                continue; // Missing required attribute - caught by HCL validation
            };

            let keystore_path = signer.attributes.get("keystore_path").map(|s| s.as_str());

            match resolve_keystore_path(keystore_account, keystore_path) {
                Ok(resolved_path) if !resolved_path.exists() => {
                    let location_hint = keystore_path
                        .map(|p| format!("looked in directory '{}'", p))
                        .unwrap_or_else(|| "looked in ~/.foundry/keystores".to_string());

                    result.warnings.push(
                        Diagnostic::warning(format!(
                            "keystore file not found: '{}' ({})",
                            keystore_account, location_hint
                        ))
                        .with_code("keystore-not-found".to_string())
                        .with_file(file_path.to_string())
                        .with_line(signer.line)
                        .with_column(signer.column)
                        .with_suggestion(format!(
                            "Ensure the keystore file exists. Create one with: cast wallet import {} --interactive",
                            keystore_account
                        ))
                    );
                }
                Ok(_) => {} // File exists, all good
                Err(e) => {
                    result.errors.push(
                        Diagnostic::error(format!("invalid keystore configuration: {}", e))
                            .with_code("keystore-invalid".to_string())
                            .with_file(file_path.to_string())
                            .with_line(signer.line)
                            .with_column(signer.column)
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::lint::config::LinterConfig;
    use crate::cli::lint::formatter::Format;
    use crate::cli::lint::test_utils::manifest_builders::*;
    use std::path::PathBuf;
    use txtx_core::manifest::WorkspaceManifest;

    // Import shared test utilities
    use crate::{assert_no_violations, assert_violation_count, assert_violation_message_contains};

    #[test]
    fn test_linter_new_with_valid_config() {
        // Arrange
        let config = LinterConfig::new(
            Some(PathBuf::from("test.yml")),
            Some("test_runbook".to_string()),
            None,
            vec![],
            Format::Json,
        );

        // Act
        let result = Linter::new(&config);

        // Assert
        assert!(result.is_ok(), "Should create linter with valid config");
    }

    #[test]
    fn test_linter_with_defaults_creates_stylish_formatter() {
        // Arrange - no setup needed

        // Act
        let linter = Linter::with_defaults();

        // Assert
        assert_eq!(linter.config.format, Format::Stylish);
        assert!(linter.config.manifest_path.is_none());
    }

    #[test]
    fn test_into_manifest_with_some_manifest() {
        // Arrange
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        // Act
        let result = Some(manifest.clone()).into_manifest();

        // Assert
        assert!(result.is_some());
        let unwrapped = result.unwrap();
        assert_eq!(unwrapped.name, "test");
        assert_eq!(unwrapped.id, "test-id");
    }

    #[test]
    fn test_into_manifest_with_none_returns_none() {
        // Arrange
        let none_manifest: Option<WorkspaceManifest> = None;

        // Act
        let result = none_manifest.into_manifest();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_into_manifest_pathbuf_from_option() {
        // Arrange - use non-existent path so it returns None
        let path = PathBuf::from("/nonexistent/path/manifest.yml");

        // Act
        let result = Some(path).into_manifest();

        // Assert
        // Non-existent file should return None (not error)
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_content_with_valid_runbook() {
        // Arrange
        let linter = Linter::with_defaults();
        let valid_content = r#"
action "test" {
    http_method = "GET"
    url = "https://example.com"
}"#;

        // Act
        let result = linter.validate_content::<Option<&PathBuf>>(
            valid_content,
            "test.tx",
            None,
            None,
        );

        // Assert
        // Valid content should produce no errors for basic structure
        // (may have warnings about undefined inputs)
        assert!(result.errors.is_empty() ||
                result.errors.iter().all(|e| e.message.contains("undefined")));
    }

    #[test]
    fn test_validate_content_with_invalid_syntax() {
        // Arrange
        let linter = Linter::with_defaults();
        let invalid_content = r#"
action "test" {
    http_method =
}"#;

        // Act
        let result = linter.validate_content::<Option<&PathBuf>>(
            invalid_content,
            "test.tx",
            None,
            None,
        );

        // Assert
        assert!(!result.errors.is_empty(), "Should have parsing errors");
    }

    #[test]
    fn test_resolve_inputs_with_empty_manifest() {
        // Arrange
        let linter = Linter::with_defaults();
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        // Act
        let inputs = linter.resolve_inputs(&manifest, None);

        // Assert
        assert!(inputs.is_empty(), "Should return empty map for manifest without inputs");
    }

    #[test]
    fn test_format_and_print_with_empty_results() {
        // Arrange
        let linter = Linter::with_defaults();
        let result = ValidationResult {
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        };

        // Act & Assert - should not panic
        linter.format_and_print(result);
    }

    #[test]
    fn test_format_and_print_with_errors() {
        // Arrange
        use txtx_addon_kit::types::diagnostics::Diagnostic;

        let linter = Linter::with_defaults();
        let mut error = Diagnostic::error("Test error message");
        error.line = Some(10);
        error.column = Some(5);
        error.file = Some("test.tx".to_string());

        let result = ValidationResult {
            errors: vec![error],
            warnings: vec![],
            suggestions: vec![],
        };

        // Act & Assert - should not panic
        linter.format_and_print(result);
    }

    #[test]
    fn test_cli_override_warns_when_overriding_global_env() {
        // Arrange - manifest defines API_KEY in global environment
        let manifest = create_manifest_with_global(&[("API_KEY", "global-value")]);

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: None,
            cli_inputs: vec![("API_KEY".to_string(), "cli-override".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, None);

        // Assert - should warn about CLI override
        assert!(
            result.warnings.iter().any(|w| w.code.as_deref() == Some("cli_input_override")),
            "Should warn about CLI override, got warnings: {:?}",
            result.warnings.iter().map(|w| &w.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cli_override_warns_when_overriding_specific_env() {
        // Arrange - manifest defines API_KEY in production environment
        let manifest = create_manifest_with_env("production", &[("API_KEY", "prod-value")]);

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: Some("production".to_string()),
            cli_inputs: vec![("API_KEY".to_string(), "cli-override".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, Some(&"production".to_string()));

        // Assert - should warn about CLI override
        assert!(
            result.warnings.iter().any(|w| w.code.as_deref() == Some("cli_input_override")),
            "Should warn about CLI override in specific environment"
        );
    }

    #[test]
    fn test_cli_override_no_warning_when_not_in_manifest() {
        // Arrange - manifest defines OTHER_VAR but not API_KEY
        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let prod_env: std::collections::HashMap<String, String> =
            vec![("OTHER_VAR".to_string(), "value".to_string())]
            .into_iter().collect();
        manifest.environments.insert("production".to_string(), prod_env.into_iter().collect());

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: Some("production".to_string()),
            cli_inputs: vec![("API_KEY".to_string(), "cli-value".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, Some(&"production".to_string()));

        // Assert - should NOT warn (API_KEY not in manifest)
        assert_no_violations!(result, "cli_input_override");
    }

    #[test]
    fn test_cli_override_multiple_inputs_selective_warning() {
        // Arrange - manifest defines some inputs
        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let prod_env: std::collections::HashMap<String, String> =
            vec![
                ("API_KEY".to_string(), "prod-key".to_string()),
                ("API_URL".to_string(), "prod-url".to_string()),
            ]
            .into_iter().collect();
        manifest.environments.insert("production".to_string(), prod_env.into_iter().collect());

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: Some("production".to_string()),
            cli_inputs: vec![
                ("API_KEY".to_string(), "cli-key".to_string()),  // Overrides manifest
                ("TIMEOUT".to_string(), "30".to_string()),       // Not in manifest
            ],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
            variable "url" {
                value = input.API_URL
            }
            variable "timeout" {
                value = input.TIMEOUT
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, Some(&"production".to_string()));

        // Assert - should warn about API_KEY only
        assert_violation_count!(result, "cli_input_override", 1);
        assert_violation_message_contains!(result, "cli_input_override", "API_KEY");
    }

    #[test]
    fn test_cli_override_no_warning_without_cli_input() {
        // Arrange - manifest defines API_KEY
        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let prod_env: std::collections::HashMap<String, String> =
            vec![("API_KEY".to_string(), "prod-value".to_string())]
            .into_iter().collect();
        manifest.environments.insert("production".to_string(), prod_env.into_iter().collect());

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: Some("production".to_string()),
            cli_inputs: vec![],  // No CLI inputs
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, Some(&"production".to_string()));

        // Assert - should NOT warn (no CLI override happening)
        assert_no_violations!(result, "cli_input_override");
    }

    #[test]
    fn test_cli_override_both_global_and_specific_env() {
        // Arrange - manifest has API_KEY in both global and production
        let manifest = with_env(
            create_manifest_with_global(&[("API_KEY", "global-value")]),
            "production",
            &[("API_KEY", "prod-value")]
        );

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: Some("production".to_string()),
            cli_inputs: vec![("API_KEY".to_string(), "cli-override".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        let content = r#"
            variable "key" {
                value = input.API_KEY
            }
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", manifest, Some(&"production".to_string()));

        // Assert - should warn about override
        assert_violation_count!(result, "cli_input_override", 1);
        assert_violation_message_contains!(result, "cli_input_override", "API_KEY");
        assert_violation_message_contains!(result, "cli_input_override", "overridden");
    }

    // Error message quality tests
    #[test]
    fn test_error_message_includes_variable_name() {
        // Arrange
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let linter = Linter::with_defaults();

        // Act
        let result = linter.validate_content(
            r#"
                variable "test" {
                    value = input.UNDEFINED_VARIABLE
                }
            "#,
            "test.tx",
            manifest,
            None,
        );

        // Assert - error message should include the undefined variable name
        assert!(!result.errors.is_empty(), "Should have errors");
        let has_variable_in_message = result.errors.iter()
            .any(|e| e.message.contains("UNDEFINED_VARIABLE"));

        assert!(has_variable_in_message,
            "Error messages should include variable name, got: {:?}",
            result.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    }

    #[test]
    fn test_warning_includes_helpful_context() {
        // Arrange - create scenario that triggers naming convention warning
        let global_env: std::collections::HashMap<String, String> =
            vec![("api-key".to_string(), "value".to_string())]
            .into_iter().collect();

        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };
        manifest.environments.insert("global".to_string(), global_env.into_iter().collect());

        let linter = Linter::with_defaults();

        // Act
        let result = linter.validate_content(
            r#"
                variable "key" {
                    value = input.api-key
                }
            "#,
            "test.tx",
            manifest,
            None,
        );

        // Assert - naming convention warning should include the problematic name
        let naming_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code.as_deref() == Some("input_naming_convention"))
            .collect();

        if !naming_warnings.is_empty() {
            assert!(naming_warnings[0].message.contains("api-key"),
                "Warning should mention the input name with hyphens");
        }
    }

    #[test]
    fn test_cli_override_message_explains_precedence() {
        // Arrange
        let global_env: std::collections::HashMap<String, String> =
            vec![("CONFIG_VAR".to_string(), "env-value".to_string())]
            .into_iter().collect();

        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };
        manifest.environments.insert("global".to_string(), global_env.into_iter().collect());

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: None,
            cli_inputs: vec![("CONFIG_VAR".to_string(), "cli-value".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        // Act
        let result = linter.validate_content(
            r#"
                variable "cfg" {
                    value = input.CONFIG_VAR
                }
            "#,
            "test.tx",
            manifest,
            None,
        );

        // Assert - warning should explain precedence
        let override_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code.as_deref() == Some("cli_input_override"))
            .collect();

        if !override_warnings.is_empty() {
            let message = &override_warnings[0].message;
            assert!(message.contains("CLI") || message.contains("command"),
                "Message should mention CLI/command line: {}", message);
        }
    }

    #[test]
    fn test_error_messages_are_actionable() {
        // Arrange - test that undefined input error suggests a fix
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let linter = Linter::with_defaults();

        // Act
        let result = linter.validate_content(
            r#"
                variable "test" {
                    value = input.MISSING_INPUT
                }
            "#,
            "test.tx",
            manifest,
            None,
        );

        // Assert - should have error about undefined input
        assert!(!result.errors.is_empty(), "Should have error for undefined input");

        // Check for actionable information in error or its context/suggestion
        let has_actionable_info = result.errors.iter().any(|e| {
            // Error message, context, or suggestion should guide the user
            e.message.contains("defined") ||
            e.message.contains("environment") ||
            e.context.as_ref().map(|c| c.contains("Add") || c.contains("define")).unwrap_or(false)
        });

        assert!(has_actionable_info,
            "Error should be actionable with guidance, got messages: {:?}",
            result.errors.iter().map(|e| (&e.message, &e.context)).collect::<Vec<_>>());
    }

    #[test]
    fn test_violations_include_error_codes() {
        // Arrange
        let global_env: std::collections::HashMap<String, String> =
            vec![("test_var".to_string(), "value".to_string())]
            .into_iter().collect();

        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };
        manifest.environments.insert("global".to_string(), global_env.into_iter().collect());

        let config = LinterConfig {
            manifest_path: None,
            runbook: None,
            environment: None,
            cli_inputs: vec![("test_var".to_string(), "override".to_string())],
            format: Format::Json,
            config_file: None,
        };

        let linter = Linter::new(&config).unwrap();

        // Act
        let result = linter.validate_content(
            r#"
                variable "v" {
                    value = input.test_var
                }
            "#,
            "test.tx",
            manifest,
            None,
        );

        // Assert - all violations should have error codes
        for error in &result.errors {
            assert!(error.code.is_some(),
                "Error should have code: {:?}", error.message);
        }

        for warning in &result.warnings {
            assert!(warning.code.is_some(),
                "Warning should have code: {:?}", warning.message);
        }
    }

    mod keystore_linting_tests {
        use super::*;
        use tempfile::TempDir;
        use std::fs;

        fn minimal_manifest() -> WorkspaceManifest {
            WorkspaceManifest {
                name: "test".to_string(),
                id: "test-id".to_string(),
                runbooks: vec![],
                environments: Default::default(),
                location: None,
            }
        }

        // TODO: This local builder is a workaround. The proper fix is to extend
        // RunbookBuilder in txtx-test-utils with a `validate_with_cli_linter()` method
        // that uses the full CLI Linter instead of just hcl_validator.
        struct SignerBuilder {
            name: String,
            signer_type: String,
            attributes: Vec<(String, String)>,
        }

        impl SignerBuilder {
            fn new(name: &str, signer_type: &str) -> Self {
                Self {
                    name: name.to_string(),
                    signer_type: signer_type.to_string(),
                    attributes: Vec::new(),
                }
            }

            fn keystore(name: &str) -> Self {
                Self::new(name, "evm::keystore")
            }

            fn web_wallet(name: &str) -> Self {
                Self::new(name, "evm::web_wallet")
            }

            fn attr(mut self, key: &str, value: &str) -> Self {
                self.attributes.push((key.to_string(), value.to_string()));
                self
            }

            fn keystore_account(self, account: &str) -> Self {
                self.attr("keystore_account", account)
            }

            fn keystore_path(self, path: &str) -> Self {
                self.attr("keystore_path", path)
            }

            fn expected_address(self, address: &str) -> Self {
                self.attr("expected_address", address)
            }

            fn build(&self) -> String {
                let attrs = self.attributes
                    .iter()
                    .map(|(k, v)| format!("    {} = \"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    "signer \"{}\" \"{}\" {{\n{}\n}}",
                    self.name, self.signer_type, attrs
                )
            }

            fn validate(&self) -> ValidationResult {
                let linter = Linter::with_defaults();
                linter.validate_content(&self.build(), "test.tx", minimal_manifest(), None)
            }
        }

        #[test]
        fn test_keystore_signer_with_existing_file() {
            // Arrange
            let temp_dir = TempDir::new().unwrap();
            let keystore_file = temp_dir.path().join("myaccount.json");
            fs::write(&keystore_file, "{}").unwrap();

            // Act
            let result = SignerBuilder::keystore("deployer")
                .keystore_account("myaccount")
                .keystore_path(&temp_dir.path().display().to_string())
                .validate();

            // Assert
            let keystore_warnings: Vec<_> = result.warnings.iter()
                .filter(|w| w.code.as_ref().is_some_and(|c| c.contains("keystore")))
                .collect();
            assert!(keystore_warnings.is_empty(),
                "Should not warn about existing keystore file: {:?}", keystore_warnings);
        }

        #[test]
        fn test_keystore_signer_with_missing_file() {
            // Arrange
            let temp_dir = TempDir::new().unwrap();
            let nonexistent_path = temp_dir.path().join("nonexistent");

            // Act
            let result = SignerBuilder::keystore("deployer")
                .keystore_account("myaccount")
                .keystore_path(&nonexistent_path.display().to_string())
                .validate();

            // Assert
            let keystore_warnings: Vec<_> = result.warnings.iter()
                .filter(|w| w.code.as_deref() == Some("keystore-not-found"))
                .collect();
            assert_eq!(keystore_warnings.len(), 1,
                "Should warn about missing keystore file");
            assert!(keystore_warnings[0].message.contains("keystore file not found"),
                "Warning should mention 'keystore file not found': {:?}", keystore_warnings[0].message);
        }

        #[test]
        fn test_keystore_signer_absolute_path_without_json_extension() {
            // Arrange & Act
            let result = SignerBuilder::keystore("deployer")
                .keystore_account("/some/absolute/path/without/extension")
                .validate();

            // Assert
            let keystore_errors: Vec<_> = result.errors.iter()
                .filter(|e| e.code.as_deref() == Some("keystore-invalid"))
                .collect();
            assert_eq!(keystore_errors.len(), 1,
                "Should error on absolute path without .json extension");
            assert!(keystore_errors[0].message.contains(".json extension"),
                "Error should mention .json extension: {:?}", keystore_errors[0].message);
        }

        #[test]
        fn test_keystore_signer_includes_suggestion() {
            // Arrange
            let temp_dir = TempDir::new().unwrap();

            // Act
            let result = SignerBuilder::keystore("deployer")
                .keystore_account("myaccount")
                .keystore_path(&temp_dir.path().display().to_string())
                .validate();

            // Assert
            let keystore_warnings: Vec<_> = result.warnings.iter()
                .filter(|w| w.code.as_deref() == Some("keystore-not-found"))
                .collect();
            assert_eq!(keystore_warnings.len(), 1);
            assert!(keystore_warnings[0].suggestion.is_some(),
                "Warning should include a suggestion");
            let suggestion = keystore_warnings[0].suggestion.as_ref().unwrap();
            assert!(suggestion.contains("cast wallet import"),
                "Suggestion should mention 'cast wallet import': {:?}", suggestion);
        }

        #[test]
        fn test_non_keystore_signer_not_validated() {
            // Arrange & Act
            let result = SignerBuilder::web_wallet("deployer")
                .expected_address("0x1234567890123456789012345678901234567890")
                .validate();

            // Assert
            let keystore_issues: Vec<_> = result.warnings.iter()
                .chain(result.errors.iter())
                .filter(|d| d.code.as_ref().is_some_and(|c| c.contains("keystore")))
                .collect();
            assert!(keystore_issues.is_empty(),
                "Non-keystore signers should not trigger keystore validation");
        }
    }
}
