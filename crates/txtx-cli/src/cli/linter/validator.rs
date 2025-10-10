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

pub struct Linter {
    config: LinterConfig,
}

impl Linter {
    pub fn new(config: &LinterConfig) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
        })
    }

    pub fn with_defaults() -> Self {
        Self {
            config: LinterConfig::default(),
        }
    }

    pub fn lint_runbook(&self, name: &str) -> Result<(), String> {
        let workspace = super::workspace::WorkspaceAnalyzer::new(&self.config)?;
        let result = workspace.analyze_runbook(name)?;

        self.format_and_print(result);
        Ok(())
    }

    pub fn lint_all(&self) -> Result<(), String> {
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
                manifest,
                environment: environment.as_ref().map(|s| s.as_str()),
                effective_inputs: &effective_inputs,
                cli_inputs: &self.config.cli_inputs,
                content,
                file_path,
                input: InputInfo {
                    name: &input_ref.name,
                    full_name: &full_name,
                },
            };

            let issues = validate_all(&context, rules);

            for issue in issues {
                match issue.severity {
                    Severity::Error => {
                        let mut diagnostic = Diagnostic::error(issue.message.into_owned())
                            .with_file(file_path.to_string())
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
                            .with_file(file_path.to_string())
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