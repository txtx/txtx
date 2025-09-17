//! HCL diagnostic extraction and conversion
//!
//! This module provides functionality to extract diagnostics from HCL parsing
//! and convert them to a format suitable for LSP and other consumers.

use super::types::{ValidationError, ValidationResult};
use std::ops::Range;

/// Represents a diagnostic from HCL parsing with full context
#[derive(Debug, Clone)]
pub struct HclDiagnostic {
    /// The error message
    pub message: String,
    /// The severity level
    pub severity: DiagnosticSeverity,
    /// The span in the source file
    pub span: Option<Range<usize>>,
    /// Additional context or suggestions
    pub hint: Option<String>,
    /// The source of the diagnostic (e.g., "hcl-parser")
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Extract diagnostics from an HCL parse error string
pub fn extract_hcl_diagnostics(error_str: &str, _source: &str) -> Vec<HclDiagnostic> {
    let mut diagnostics = Vec::new();

    // Extract the main error
    let diagnostic = HclDiagnostic {
        message: error_str.to_string(),
        severity: DiagnosticSeverity::Error,
        span: extract_span_from_error_str(error_str),
        hint: extract_hint_from_error_str(error_str),
        source: "hcl-parser".to_string(),
    };

    diagnostics.push(diagnostic);

    diagnostics
}

/// Parse HCL content and return both the result and any diagnostics
pub fn parse_with_diagnostics(
    content: &str,
    _file_path: &str,
) -> (Result<txtx_addon_kit::hcl::structure::Body, String>, Vec<HclDiagnostic>) {
    use std::str::FromStr;

    let mut diagnostics = Vec::new();

    let result = txtx_addon_kit::hcl::structure::Body::from_str(content).map_err(|e| {
        let error_str = e.to_string();
        // Extract diagnostics from the error
        diagnostics.extend(extract_hcl_diagnostics(&error_str, content));
        format!("Failed to parse runbook: {}", error_str)
    });

    (result, diagnostics)
}

/// Enhanced validation that includes HCL diagnostics
pub fn validate_with_diagnostics(
    content: &str,
    file_path: &str,
) -> (ValidationResult, Vec<HclDiagnostic>) {
    let mut result = ValidationResult::new();
    let mut hcl_diagnostics = Vec::new();

    // First, try to parse with diagnostics
    let (parse_result, parse_diagnostics) = parse_with_diagnostics(content, file_path);
    hcl_diagnostics.extend(parse_diagnostics);

    match parse_result {
        Ok(_body) => {
            // If parsing succeeded, run validation
            if let Err(e) = super::hcl_validator::validate_with_hcl(content, &mut result, file_path)
            {
                // Add any validation errors as diagnostics
                let diagnostic = HclDiagnostic {
                    message: e,
                    severity: DiagnosticSeverity::Error,
                    span: None,
                    hint: None,
                    source: "hcl-validator".to_string(),
                };
                hcl_diagnostics.push(diagnostic);
            }
        }
        Err(e) => {
            // Parsing failed, add to validation result
            let error = ValidationError {
                message: e.clone(),
                file: file_path.to_string(),
                line: Some(0),
                column: Some(0),
                context: None,
                documentation_link: None,
            };
            result.errors.push(error);
        }
    }

    (result, hcl_diagnostics)
}

// Helper functions

fn extract_span_from_error_str(_error_str: &str) -> Option<Range<usize>> {
    // TODO: Implement proper span extraction from HCL error string
    // This requires parsing the error message for position info
    None
}

fn extract_hint_from_error_str(_error_str: &str) -> Option<String> {
    // TODO: Extract helpful hints from the error message
    // For example, suggestions for fixing syntax errors
    None
}

/// Convert line/column to byte offset in source
pub fn position_to_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut current_line = 1;
    let mut current_column = 1;

    for (offset, ch) in source.char_indices() {
        if current_line == line && current_column == column {
            return Some(offset);
        }

        if ch == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    None
}

/// Convert byte offset to line/column in source
pub fn offset_to_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_conversions() {
        let source = "line1\nline2\nline3";

        // Test position to offset
        assert_eq!(position_to_offset(source, 1, 1), Some(0));
        assert_eq!(position_to_offset(source, 2, 1), Some(6));
        assert_eq!(position_to_offset(source, 3, 1), Some(12));

        // Test offset to position
        assert_eq!(offset_to_position(source, 0), (1, 1));
        assert_eq!(offset_to_position(source, 6), (2, 1));
        assert_eq!(offset_to_position(source, 12), (3, 1));
    }

    #[test]
    fn test_diagnostic_severity() {
        // Test that severity enum values are distinct
        assert_ne!(DiagnosticSeverity::Error as u8, DiagnosticSeverity::Warning as u8);
        assert_ne!(DiagnosticSeverity::Warning as u8, DiagnosticSeverity::Information as u8);
        assert_ne!(DiagnosticSeverity::Information as u8, DiagnosticSeverity::Hint as u8);
    }

    #[test]
    fn test_extract_hcl_diagnostics() {
        let error_str = "Parse error: unexpected token";
        let diagnostics = extract_hcl_diagnostics(error_str, "test content");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, error_str);
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostics[0].source, "hcl-parser");
    }

    #[test]
    fn test_validation_result_integration() {
        let mut result = ValidationResult::new();
        assert!(result.errors.is_empty());

        // Add an error
        result.errors.push(ValidationError {
            message: "Test error".to_string(),
            file: "test.tx".to_string(),
            line: Some(1),
            column: Some(1),
            context: None,
            documentation_link: None,
        });

        assert_eq!(result.errors.len(), 1);
    }
}
