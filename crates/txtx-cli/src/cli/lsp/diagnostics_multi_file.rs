//! Multi-file aware diagnostics for LSP
//!
//! This module provides diagnostics that understand multi-file runbooks

use crate::cli::common::addon_registry;
use crate::cli::doctor::{
    CliInputOverrideRule, InputDefinedRule, InputNamingConventionRule, SensitiveDataRule,
    ValidationContext, ValidationRule,
};
use crate::cli::lsp::multi_file::{
    get_runbook_name_for_file, load_multi_file_runbook, map_line_to_file,
};
use crate::cli::lsp::validation::validation_outcome_to_diagnostic;
use crate::cli::lsp::workspace::manifest_converter::lsp_manifest_to_workspace_manifest;
use crate::cli::lsp::workspace::Manifest;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::collections::HashMap;
use txtx_core::validation::ValidationResult;

/// Validate a file that may be part of a multi-file runbook
pub fn validate_with_multi_file_support(
    file_uri: &Url,
    content: &str,
    lsp_manifest: Option<&Manifest>,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> Vec<Diagnostic> {
    eprintln!("[DEBUG] validate_with_multi_file_support called for: {}", file_uri);

    // Check if this file is part of a multi-file runbook
    if let Some(manifest) = lsp_manifest {
        eprintln!("[DEBUG] Manifest found, checking for runbook name");
        if let Some(runbook_name) = get_runbook_name_for_file(file_uri, manifest) {
            eprintln!("[DEBUG] Found runbook name: {}", runbook_name);
            // Find the runbook in the manifest
            if let Some(runbook) = manifest.runbooks.iter().find(|r| r.name == runbook_name) {
                // Check if the runbook location is a directory
                if let Ok(manifest_path) = manifest.uri.to_file_path() {
                    let runbook_path = manifest_path
                        .parent()
                        .map(|p| p.join(&runbook.location))
                        .unwrap_or_else(|| runbook.location.clone().into());

                    eprintln!(
                        "[DEBUG] Runbook path: {:?}, is_dir: {}",
                        runbook_path,
                        runbook_path.is_dir()
                    );
                    if runbook_path.is_dir() {
                        eprintln!("[DEBUG] This is a multi-file runbook, calling validate_multi_file_runbook");
                        // This is a multi-file runbook
                        return validate_multi_file_runbook(
                            file_uri,
                            &runbook_path,
                            &runbook_name,
                            manifest,
                            environment,
                            cli_inputs,
                        );
                    } else {
                        eprintln!("[DEBUG] Not a directory, falling back to single file");
                    }
                }
            }
        }
    }

    // Fall back to single-file validation
    crate::cli::lsp::diagnostics_enhanced::validate_runbook_with_doctor_rules(
        file_uri,
        content,
        lsp_manifest,
        environment,
        cli_inputs,
    )
}

/// Validate a multi-file runbook and return diagnostics for the current file
fn validate_multi_file_runbook(
    current_file_uri: &Url,
    runbook_dir: &std::path::Path,
    runbook_name: &str,
    lsp_manifest: &Manifest,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> Vec<Diagnostic> {
    // Load the multi-file runbook
    let multi_file = match load_multi_file_runbook(runbook_dir, runbook_name, environment) {
        Ok(mf) => mf,
        Err(e) => {
            // Return error diagnostic
            return vec![Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx".to_string()),
                message: format!("Failed to load multi-file runbook: {}", e),
                related_information: None,
                tags: None,
                data: None,
            }];
        }
    };

    let mut diagnostics = Vec::new();
    let mut validation_result = ValidationResult::new();

    // Load addon specifications
    let addons = addon_registry::get_all_addons();
    let addon_specs = addon_registry::extract_addon_specifications(&addons);

    // Run HCL validation on combined content
    let input_refs = match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
        &multi_file.combined_content,
        &mut validation_result,
        &runbook_dir.to_string_lossy(),
        addon_specs,
    ) {
        Ok(refs) => refs,
        Err(_) => Vec::new(),
    };

    // Convert validation errors, mapping them back to original files
    let current_file_path = current_file_uri.path();

    for error in &validation_result.errors {
        if let Some(line) = error.line {
            if let Some((file_path, mapped_line)) =
                map_line_to_file(line, &multi_file.file_boundaries)
            {
                // Only include errors for the current file
                if file_path == current_file_path {
                    let range = Range {
                        start: Position {
                            line: mapped_line.saturating_sub(1) as u32,
                            character: error.column.unwrap_or(1).saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: mapped_line.saturating_sub(1) as u32,
                            character: (error.column.unwrap_or(1) + 20) as u32,
                        },
                    };

                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("txtx".to_string()),
                        message: error.message.clone(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
        }
    }

    // Do the same for warnings
    for warning in &validation_result.warnings {
        if let Some(line) = warning.line {
            if let Some((file_path, mapped_line)) =
                map_line_to_file(line, &multi_file.file_boundaries)
            {
                if file_path == current_file_path {
                    let range = Range {
                        start: Position {
                            line: mapped_line.saturating_sub(1) as u32,
                            character: warning.column.unwrap_or(1).saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: mapped_line.saturating_sub(1) as u32,
                            character: (warning.column.unwrap_or(1) + 20) as u32,
                        },
                    };

                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: None,
                        code_description: None,
                        source: Some("txtx".to_string()),
                        message: warning.message.clone(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
        }
    }

    // Run doctor validation rules
    let workspace_manifest = lsp_manifest_to_workspace_manifest(lsp_manifest);

    let effective_inputs = get_effective_inputs(&workspace_manifest, environment);

    // Create validation rules
    let rules: Vec<Box<dyn ValidationRule>> = vec![
        Box::new(InputDefinedRule),
        Box::new(InputNamingConventionRule),
        Box::new(CliInputOverrideRule),
        Box::new(SensitiveDataRule),
    ];

    // Run validation rules for each input reference
    for input_ref in &input_refs {
        // Map the input reference line back to original file
        if let Some((file_path, mapped_line)) =
            map_line_to_file(input_ref.line, &multi_file.file_boundaries)
        {
            // Only process refs from the current file
            if file_path == current_file_path {
                let input_name = input_ref.name.strip_prefix("input.").unwrap_or(&input_ref.name);

                // Create validation context
                let ctx = ValidationContext {
                    input_name,
                    full_name: &input_ref.name,
                    manifest: &workspace_manifest,
                    environment,
                    effective_inputs: &effective_inputs,
                    cli_inputs,
                    content: &multi_file.combined_content,
                    file_path: &runbook_dir.to_string_lossy(),
                };

                // Run each rule
                for rule in &rules {
                    let outcome = rule.check(&ctx);

                    // Convert outcome to diagnostic at the mapped location
                    let range = Range {
                        start: Position {
                            line: (mapped_line.saturating_sub(1)) as u32,
                            character: input_ref.column as u32,
                        },
                        end: Position {
                            line: (mapped_line.saturating_sub(1)) as u32,
                            character: (input_ref.column + input_ref.name.len()) as u32,
                        },
                    };

                    if let Some(diagnostic) = validation_outcome_to_diagnostic(outcome, range) {
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

/// Get effective inputs for an environment
fn get_effective_inputs(
    manifest: &txtx_core::manifest::WorkspaceManifest,
    environment: Option<&str>,
) -> HashMap<String, String> {
    let mut effective_inputs = HashMap::new();

    // First, add global inputs
    if let Some(global_env) = manifest.environments.get("global") {
        for (key, value) in global_env {
            effective_inputs.insert(key.clone(), value.clone());
        }
    }

    // Then, override with environment-specific inputs
    if let Some(env_name) = environment {
        if let Some(env) = manifest.environments.get(env_name) {
            for (key, value) in env {
                effective_inputs.insert(key.clone(), value.clone());
            }
        }
    }

    effective_inputs
}
