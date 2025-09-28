//! Conversion utilities between linter and LSP types

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use txtx_core::validation::{ValidationError, ValidationWarning};

/// Convert a validation error to an LSP diagnostic
#[allow(dead_code)]
pub fn error_to_diagnostic(error: &ValidationError) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: error.line.unwrap_or(0).saturating_sub(1) as u32,
                character: error.column.unwrap_or(0).saturating_sub(1) as u32,
            },
            end: Position {
                line: error.line.unwrap_or(0).saturating_sub(1) as u32,
                character: error.column.unwrap_or(0) as u32,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: error.documentation_link.as_ref().map(|link| {
            lsp_types::CodeDescription {
                href: lsp_types::Url::parse(link).ok().unwrap_or_else(|| {
                    lsp_types::Url::parse("https://docs.txtx.io/linter").unwrap()
                }),
            }
        }),
        source: Some("txtx-linter".to_string()),
        message: format!(
            "{}{}",
            error.message,
            error.context.as_ref()
                .map(|ctx| format!("\n\n{}", ctx))
                .unwrap_or_default()
        ),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert a validation warning to an LSP diagnostic
#[allow(dead_code)]
pub fn warning_to_diagnostic(warning: &ValidationWarning) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: warning.line.unwrap_or(0).saturating_sub(1) as u32,
                character: warning.column.unwrap_or(0).saturating_sub(1) as u32,
            },
            end: Position {
                line: warning.line.unwrap_or(0).saturating_sub(1) as u32,
                character: warning.column.unwrap_or(0) as u32,
            },
        },
        severity: Some(DiagnosticSeverity::WARNING),
        code: None,
        code_description: None,
        source: Some("txtx-linter".to_string()),
        message: format!(
            "{}{}",
            warning.message,
            warning.suggestion.as_ref()
                .map(|sug| format!("\n\nSuggestion: {}", sug))
                .unwrap_or_default()
        ),
        related_information: None,
        tags: None,
        data: None,
    }
}