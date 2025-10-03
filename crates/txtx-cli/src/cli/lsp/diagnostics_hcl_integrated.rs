//! HCL-integrated diagnostics for the txtx Language Server
//!
//! This module provides enhanced diagnostics that leverage HCL parser's
//! native diagnostic capabilities per ADR-002.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

use super::validation::validation_errors_to_diagnostics;
use crate::cli::common::addon_registry;

/// Validate a runbook file using integrated HCL diagnostics
#[allow(dead_code)]
pub fn validate_runbook_with_hcl(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
    let mut all_diagnostics = Vec::new();
    let file_path = file_uri.path();

    // First, try to parse the HCL and get any syntax errors
    match txtx_addon_kit::hcl::structure::Body::from_str(content) {
        Ok(_body) => {
            // Parsing succeeded, now run semantic validation
            let mut validation_result = txtx_core::validation::ValidationResult::new();

            // Load addon specifications
            let addons = addon_registry::get_all_addons();
            let addon_specs = addon_registry::extract_addon_specifications(&addons);

            // Run validation
            match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
                content,
                &mut validation_result,
                file_path,
                addon_specs,
            ) {
                Ok(_) => {
                    // Convert validation results to diagnostics
                    all_diagnostics.extend(validation_errors_to_diagnostics(
                        &validation_result.errors,
                        file_uri,
                    ));

                    // Also add warnings as diagnostics
                    for warning in &validation_result.warnings {
                        let range = Range {
                            start: Position {
                                line: warning.line.unwrap_or(1).saturating_sub(1) as u32,
                                character: warning.column.unwrap_or(0) as u32,
                            },
                            end: Position {
                                line: warning.line.unwrap_or(1).saturating_sub(1) as u32,
                                character: (warning.column.unwrap_or(0).saturating_add(10)) as u32, // Approximate end
                            },
                        };

                        all_diagnostics.push(Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: None,
                            code_description: None,
                            source: Some("txtx-validator".to_string()),
                            message: warning.message.clone(),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
                Err(parse_error) => {
                    // Validation failed - add as error
                    all_diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("txtx-validator".to_string()),
                        message: parse_error,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
        }
        Err(parse_error) => {
            // HCL parsing failed - extract detailed error information
            let error_str = parse_error.to_string();

            // Try to extract line/column information from the error message
            // HCL errors often include position information
            let (line, column) = extract_position_from_error(&error_str);

            let range = Range {
                start: Position {
                    line: line.saturating_sub(1) as u32,
                    character: column.saturating_sub(1) as u32,
                },
                end: Position {
                    line: line.saturating_sub(1) as u32,
                    character: (column + 20) as u32,
                },
            };

            all_diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("hcl-parser".to_string()),
                message: format!("HCL parse error: {}", error_str),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    all_diagnostics
}

/// Extract line and column from HCL error messages
///
/// HCL errors often contain position information in formats like:
/// - "line 5, column 10"
/// - "at 5:10"
/// - "on line 5"
#[allow(dead_code)]
pub fn extract_position_from_error(error_msg: &str) -> (usize, usize) {
    // Try to find line number
    let line = if let Some(pos) = error_msg.find("line ") {
        let start = pos + 5;
        error_msg[start..]
            .chars()
            .take_while(|c| c.is_numeric())
            .collect::<String>()
            .parse()
            .unwrap_or(1)
    } else if error_msg.contains(':') {
        // Try format like "5:10"
        error_msg
            .split_whitespace()
            .find(|s| s.contains(':'))
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1)
    } else {
        1
    };

    // Try to find column number
    let column = if let Some(pos) = error_msg.find("column ") {
        let start = pos + 7;
        error_msg[start..]
            .chars()
            .take_while(|c| c.is_numeric())
            .collect::<String>()
            .parse()
            .unwrap_or(1)
    } else if error_msg.contains(':') {
        // Try format like "5:10"
        error_msg
            .split_whitespace()
            .find(|s| s.contains(':'))
            .and_then(|s| s.split(':').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(1)
    } else {
        1
    };

    (line, column)
}

use std::str::FromStr;
