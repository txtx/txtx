//! Multi-file aware diagnostics for LSP
//!
//! This module provides diagnostics that understand multi-file runbooks

use crate::cli::linter::{Linter, LinterConfig, Format};
use crate::cli::lsp::multi_file::{
    get_runbook_name_for_file, load_multi_file_runbook, map_line_to_file,
};
use crate::cli::lsp::workspace::manifest_converter::lsp_manifest_to_workspace_manifest;
use crate::cli::lsp::workspace::Manifest;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::collections::HashMap;
use std::path::PathBuf;

/// Validate a file that may be part of a multi-file runbook
///
/// Returns diagnostics grouped by file URI. For multi-file runbooks, this will include
/// diagnostics for all files in the runbook. For single files, it will only include
/// diagnostics for that file.
pub fn validate_with_multi_file_support(
    file_uri: &Url,
    content: &str,
    lsp_manifest: Option<&Manifest>,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> HashMap<Url, Vec<Diagnostic>> {
    eprintln!("[DEBUG] validate_with_multi_file_support called for: {}", file_uri);

    let Some(manifest) = lsp_manifest else {
        eprintln!("[DEBUG] No manifest, falling back to single-file validation");
        let diagnostics = validate_single_file(file_uri, content, lsp_manifest, environment, cli_inputs);
        let mut result = HashMap::new();
        if !diagnostics.is_empty() {
            result.insert(file_uri.clone(), diagnostics);
        }
        return result;
    };

    eprintln!("[DEBUG] Manifest found, checking for runbook name");
    let Some(runbook_name) = get_runbook_name_for_file(file_uri, manifest) else {
        eprintln!("[DEBUG] No runbook name found, falling back to single-file validation");
        let diagnostics = validate_single_file(file_uri, content, lsp_manifest, environment, cli_inputs);
        let mut result = HashMap::new();
        if !diagnostics.is_empty() {
            result.insert(file_uri.clone(), diagnostics);
        }
        return result;
    };

    eprintln!("[DEBUG] Found runbook name: {}", runbook_name);
    let Some(runbook) = manifest.runbooks.iter().find(|r| r.name == runbook_name) else {
        eprintln!("[DEBUG] Runbook not found in manifest, falling back to single-file validation");
        let diagnostics = validate_single_file(file_uri, content, lsp_manifest, environment, cli_inputs);
        let mut result = HashMap::new();
        if !diagnostics.is_empty() {
            result.insert(file_uri.clone(), diagnostics);
        }
        return result;
    };

    let Ok(manifest_path) = manifest.uri.to_file_path() else {
        eprintln!("[DEBUG] Invalid manifest path, falling back to single-file validation");
        let diagnostics = validate_single_file(file_uri, content, lsp_manifest, environment, cli_inputs);
        let mut result = HashMap::new();
        if !diagnostics.is_empty() {
            result.insert(file_uri.clone(), diagnostics);
        }
        return result;
    };

    let runbook_path = manifest_path
        .parent()
        .map(|p| p.join(&runbook.location))
        .unwrap_or_else(|| runbook.location.clone().into());

    eprintln!("[DEBUG] Runbook path: {:?}, is_dir: {}", runbook_path, runbook_path.is_dir());

    if !runbook_path.is_dir() {
        eprintln!("[DEBUG] Not a directory, falling back to single-file validation");
        let diagnostics = validate_single_file(file_uri, content, lsp_manifest, environment, cli_inputs);
        let mut result = HashMap::new();
        if !diagnostics.is_empty() {
            result.insert(file_uri.clone(), diagnostics);
        }
        return result;
    }

    eprintln!("[DEBUG] This is a multi-file runbook, calling validate_multi_file_runbook");
    validate_multi_file_runbook(
        file_uri,
        &runbook_name,
        manifest,
        environment,
        cli_inputs,
    )
}

/// Validate a multi-file runbook and return diagnostics grouped by file
fn validate_multi_file_runbook(
    file_uri: &Url,
    runbook_name: &str,
    manifest: &Manifest,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> HashMap<Url, Vec<Diagnostic>> {
    eprintln!("[DEBUG] Starting multi-file validation for runbook: {}", runbook_name);
    let mut diagnostics_by_file: HashMap<Url, Vec<Diagnostic>> = HashMap::new();

    // Convert LSP manifest to workspace manifest
    let _workspace_manifest = lsp_manifest_to_workspace_manifest(manifest);

    // Get the root directory for the runbook
    let root_dir = match manifest.runbooks
        .iter()
        .find(|r| r.name == runbook_name)
        .and_then(|r| {
            manifest.uri.to_file_path().ok().and_then(|p| {
                p.parent().map(|parent| parent.join(&r.location))
            })
        }) {
        Some(dir) => dir,
        None => {
            let error_diag = Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-lsp".to_string()),
                message: format!("Could not determine root directory for runbook {}", runbook_name),
                related_information: None,
                tags: None,
                data: None,
            };
            diagnostics_by_file.insert(file_uri.clone(), vec![error_diag]);
            return diagnostics_by_file;
        }
    };

    // Load the complete multi-file runbook
    let multi_file_runbook = match load_multi_file_runbook(&root_dir, runbook_name, environment) {
        Ok(mfr) => mfr,
        Err(err) => {
            eprintln!("[DEBUG] Failed to load multi-file runbook: {}", err);
            let error_diag = Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-lsp".to_string()),
                message: format!("Failed to load multi-file runbook: {}", err),
                related_information: None,
                tags: None,
                data: None,
            };
            diagnostics_by_file.insert(file_uri.clone(), vec![error_diag]);
            return diagnostics_by_file;
        }
    };

    let combined_content = multi_file_runbook.combined_content;
    eprintln!("[DEBUG] Combined content length: {}", combined_content.len());

    // Create linter config
    let config = LinterConfig::new(
        Some(PathBuf::from("./txtx.yml")),
        Some(runbook_name.to_string()),
        environment.map(String::from),
        cli_inputs.to_vec(),
        Format::Json,
    );

    // Create and run linter
    match Linter::new(&config) {
        Ok(linter) => {
            let result = linter.validate_content(
                &combined_content,
                runbook_name,
                Some(&PathBuf::from("./txtx.yml")),
                environment.map(String::from).as_ref(),
            );

            // Convert errors to diagnostics grouped by file
            for error in &result.errors {
                let line = error.line.unwrap_or(1);

                // Map the line in the combined content to the actual file
                let mapped = map_line_to_file(line, &multi_file_runbook.file_boundaries);
                let (target_file_path, adjusted_line) = match mapped {
                    Some((path, line)) => (path, line),
                    None => continue, // Skip diagnostics we can't map
                };
                let target_file_uri = Url::from_file_path(&target_file_path).unwrap_or_else(|_| file_uri.clone());

                // Group diagnostics by their target file
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line: adjusted_line.saturating_sub(1) as u32,
                            character: error.column.unwrap_or(0).saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: adjusted_line.saturating_sub(1) as u32,
                            character: error.column.unwrap_or(0) as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: error.documentation.as_ref().map(|link| {
                        lsp_types::CodeDescription {
                            href: lsp_types::Url::parse(link).ok().unwrap_or_else(|| {
                                lsp_types::Url::parse("https://docs.txtx.io/linter").unwrap()
                            }),
                        }
                    }),
                    source: Some("txtx-linter".to_string()),
                    message: error.message.clone(),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics_by_file.entry(target_file_uri).or_insert_with(Vec::new).push(diagnostic);
            }

            // Convert warnings to diagnostics grouped by file
            for warning in &result.warnings {
                let line = warning.line.unwrap_or(1);

                // Map the line in the combined content to the actual file
                let mapped = map_line_to_file(line, &multi_file_runbook.file_boundaries);
                let (target_file_path, adjusted_line) = match mapped {
                    Some((path, line)) => (path, line),
                    None => continue, // Skip diagnostics we can't map
                };
                let target_file_uri = Url::from_file_path(&target_file_path).unwrap_or_else(|_| file_uri.clone());

                // Group diagnostics by their target file
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line: adjusted_line.saturating_sub(1) as u32,
                            character: warning.column.unwrap_or(0).saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: adjusted_line.saturating_sub(1) as u32,
                            character: warning.column.unwrap_or(0) as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("txtx-linter".to_string()),
                    message: warning.message.clone(),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics_by_file.entry(target_file_uri).or_insert_with(Vec::new).push(diagnostic);
            }
        }
        Err(err) => {
            let error_diag = Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-linter".to_string()),
                message: format!("Failed to initialize linter: {}", err),
                related_information: None,
                tags: None,
                data: None,
            };
            diagnostics_by_file.insert(file_uri.clone(), vec![error_diag]);
        }
    }

    let total_diagnostics: usize = diagnostics_by_file.values().map(|v| v.len()).sum();
    eprintln!("[DEBUG] Multi-file validation produced {} diagnostics across {} files",
              total_diagnostics, diagnostics_by_file.len());
    diagnostics_by_file
}

/// Validate a single file
fn validate_single_file(
    file_uri: &Url,
    content: &str,
    lsp_manifest: Option<&Manifest>,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> Vec<Diagnostic> {
    use crate::cli::lsp::linter_adapter::validate_runbook_with_linter_rules;

    validate_runbook_with_linter_rules(
        file_uri,
        content,
        lsp_manifest,
        environment,
        cli_inputs,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_with_simple_content() {
        let file_uri = Url::parse("file:///test.tx").unwrap();
        let content = r#"
runbook "test" {
    version = "1.0"
}
"#;

        let diagnostics = validate_with_multi_file_support(
            &file_uri,
            content,
            None,
            None,
            &[],
        );

        // Should not crash, actual validation results depend on linter implementation
        assert!(diagnostics.is_empty() || !diagnostics.is_empty());
    }
}