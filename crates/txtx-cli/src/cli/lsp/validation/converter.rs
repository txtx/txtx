//! Conversion utilities between linter and LSP types

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range};
use txtx_core::validation::Diagnostic;
use txtx_addon_kit::types::diagnostics::DiagnosticLevel;

/// Convert a validation diagnostic to an LSP diagnostic
#[allow(dead_code)]
pub fn diagnostic_to_lsp(diag: &Diagnostic) -> LspDiagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Note => DiagnosticSeverity::INFORMATION,
    };

    LspDiagnostic {
        range: Range {
            start: Position {
                line: diag.line.unwrap_or(0).saturating_sub(1) as u32,
                character: diag.column.unwrap_or(0).saturating_sub(1) as u32,
            },
            end: Position {
                line: diag.line.unwrap_or(0).saturating_sub(1) as u32,
                character: diag.column.unwrap_or(0) as u32,
            },
        },
        severity: Some(severity),
        code: None,
        code_description: diag.documentation.as_ref().map(|link| {
            lsp_types::CodeDescription {
                href: lsp_types::Url::parse(link).ok().unwrap_or_else(|| {
                    lsp_types::Url::parse("https://docs.txtx.io/linter").unwrap()
                }),
            }
        }),
        source: Some("txtx-linter".to_string()),
        message: format!(
            "{}{}{}",
            diag.message,
            diag.context.as_ref()
                .map(|ctx| format!("\n\n{}", ctx))
                .unwrap_or_default(),
            diag.suggestion.as_ref()
                .map(|sug| format!("\n\nSuggestion: {}", sug))
                .unwrap_or_default()
        ),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert a validation error to an LSP diagnostic (deprecated alias)
#[allow(dead_code)]
#[deprecated(note = "Use diagnostic_to_lsp instead")]
pub fn error_to_diagnostic(error: &Diagnostic) -> LspDiagnostic {
    diagnostic_to_lsp(error)
}

/// Convert a validation warning to an LSP diagnostic (deprecated alias)
#[allow(dead_code)]
#[deprecated(note = "Use diagnostic_to_lsp instead")]
pub fn warning_to_diagnostic(warning: &Diagnostic) -> LspDiagnostic {
    diagnostic_to_lsp(warning)
}