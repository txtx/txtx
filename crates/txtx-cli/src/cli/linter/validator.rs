//! Linter validation engine
//!
//! # C4 Architecture Annotations
//! @c4-component Linter Engine
//! @c4-container Lint Command
//! @c4-description Orchestrates validation using ValidationContext from core
//! @c4-description Uses same validation pipeline for single and multi-file (normalized) content
//! @c4-technology Rust
//! @c4-uses ValidationContext "Creates with config"
//! @c4-uses FileBoundaryMapper "Maps errors to source files (multi-file only)"
//! @c4-uses Formatter "Formats results"

use std::path::PathBuf;
use txtx_core::validation::{ValidationResult, Diagnostic};
use txtx_core::manifest::WorkspaceManifest;
use txtx_addon_kit::helpers::fs::FileLocation;
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
            Ok(input_refs) => {
                if let Some(ref manifest) = manifest {
                    self.validate_with_rules(&input_refs, content, file_path, manifest, environment, &mut result);
                }
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
                manifest: manifest.clone(),
                environment: environment.cloned(),
                effective_inputs: effective_inputs.clone(),
                cli_inputs: self.config.cli_inputs.clone(),
                content: content.to_string(),
                file_path: file_path.to_string(),
                input: InputInfo {
                    name: input_ref.name.clone(),
                    full_name,
                },
            };

            let issues = validate_all(&context, rules);

            for issue in issues {
                match issue.severity {
                    Severity::Error => {
                        let mut diagnostic = Diagnostic::error(issue.message.into_owned())
                            .with_code(issue.rule)
                            .with_file(file_path)
                            .with_line(input_ref.line)
                            .with_column(input_ref.column);

                        if let Some(help) = issue.help {
                            diagnostic = diagnostic.with_context(help.into_owned());
                        }

                        if let Some(example) = issue.example {
                            diagnostic = diagnostic.with_documentation(example);
                        }

                        result.errors.push(diagnostic);
                    }
                    Severity::Warning => {
                        let mut diagnostic = Diagnostic::warning(issue.message.into_owned())
                            .with_code(issue.rule)
                            .with_file(file_path)
                            .with_line(input_ref.line)
                            .with_column(input_ref.column);

                        if let Some(help) = issue.help {
                            diagnostic = diagnostic.with_suggestion(help.into_owned());
                        }

                        result.warnings.push(diagnostic);
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::linter::config::LinterConfig;
    use crate::cli::linter::formatter::Format;
    use std::path::PathBuf;
    use txtx_core::manifest::WorkspaceManifest;

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
}