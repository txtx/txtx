//! Unified diagnostic conversion utilities
//!
//! This module provides a single source of truth for converting validation
//! diagnostics to LSP diagnostic format.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use txtx_addon_kit::types::diagnostics::DiagnosticLevel;
use txtx_core::validation::Diagnostic as CoreDiagnostic;

/// Convert a core validation diagnostic to LSP diagnostic format
pub fn to_lsp_diagnostic(diag: &CoreDiagnostic) -> Diagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Note => DiagnosticSeverity::INFORMATION,
    };

    let range = create_diagnostic_range(
        diag.line.unwrap_or(1),
        diag.column.unwrap_or(1),
        estimate_token_length(&diag.message),
    );

    Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        code_description: diag.documentation.as_ref().map(|link| {
            lsp_types::CodeDescription {
                href: lsp_types::Url::parse(link)
                    .ok()
                    .unwrap_or_else(|| {
                        lsp_types::Url::parse("https://docs.txtx.io/linter").unwrap()
                    }),
            }
        }),
        source: Some("txtx-linter".to_string()),
        message: build_message(diag),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Create a Range from line, column, and estimated token length
///
/// LSP uses 0-based line numbers, but our diagnostics use 1-based.
/// This function handles the conversion.
fn create_diagnostic_range(line: usize, column: usize, length: usize) -> Range {
    Range {
        start: Position {
            line: line.saturating_sub(1) as u32,
            character: column.saturating_sub(1) as u32,
        },
        end: Position {
            line: line.saturating_sub(1) as u32,
            character: column.saturating_sub(1).saturating_add(length) as u32,
        },
    }
}

/// Build the complete diagnostic message including context and suggestions
fn build_message(diag: &CoreDiagnostic) -> String {
    let mut message = diag.message.clone();

    if let Some(context) = &diag.context {
        message.push_str("\n\n");
        message.push_str(context);
    }

    if let Some(suggestion) = &diag.suggestion {
        message.push_str("\n\nSuggestion: ");
        message.push_str(suggestion);
    }

    message
}

/// Estimate token length from diagnostic message
///
/// Looks for quoted identifiers in the message. Falls back to a default length.
fn estimate_token_length(message: &str) -> usize {
    // Look for quoted identifiers in the message
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            return end.min(50); // Cap at reasonable length
        }
    }

    // Default: 8 characters
    8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_diagnostic_range() {
        let range = create_diagnostic_range(5, 10, 8);
        assert_eq!(range.start.line, 4); // 0-based
        assert_eq!(range.start.character, 9); // 0-based
        assert_eq!(range.end.line, 4);
        assert_eq!(range.end.character, 17); // start + length
    }

    #[test]
    fn test_estimate_token_length() {
        assert_eq!(estimate_token_length("Error in 'my_variable'"), 11);
        assert_eq!(estimate_token_length("Error without quotes"), 8);
    }

    #[test]
    fn test_build_message_with_context() {
        let diag = CoreDiagnostic::error("Test error")
            .with_context("Additional context".to_string());

        let message = build_message(&diag);
        assert!(message.contains("Test error"));
        assert!(message.contains("Additional context"));
    }

    #[test]
    fn test_build_message_with_suggestion() {
        let diag = CoreDiagnostic::error("Test error")
            .with_suggestion("Try this instead".to_string());

        let message = build_message(&diag);
        assert!(message.contains("Test error"));
        assert!(message.contains("Suggestion: Try this instead"));
    }
}
