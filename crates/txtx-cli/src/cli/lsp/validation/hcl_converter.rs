//! Convert HCL diagnostics to LSP diagnostic format

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use txtx_core::validation::hcl_diagnostics::{DiagnosticSeverity as HclSeverity, HclDiagnostic};

/// Convert an HCL diagnostic to LSP diagnostic format
#[allow(dead_code)]
pub fn hcl_to_lsp_diagnostic(hcl_diag: &HclDiagnostic, source: &str) -> Diagnostic {
    // Convert span to LSP range
    let range = if let Some(span) = &hcl_diag.span {
        span_to_range(source, span.start, span.end)
    } else {
        // Default to first line if no span available
        Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }
    };

    // Convert severity
    let severity = match hcl_diag.severity {
        HclSeverity::Error => DiagnosticSeverity::ERROR,
        HclSeverity::Warning => DiagnosticSeverity::WARNING,
        HclSeverity::Information => DiagnosticSeverity::INFORMATION,
        HclSeverity::Hint => DiagnosticSeverity::HINT,
    };

    // Build the diagnostic
    let mut diagnostic = Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        code_description: None,
        source: Some(hcl_diag.source.clone()),
        message: hcl_diag.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    };

    // Add hint as related information if available
    if let Some(hint) = &hcl_diag.hint {
        // For now, append hint to message
        // In future, could use related_information
        diagnostic.message = format!("{}\n\nHint: {}", diagnostic.message, hint);
    }

    diagnostic
}

/// Convert a byte span to LSP range
#[allow(dead_code)]
fn span_to_range(source: &str, start: usize, end: usize) -> Range {
    let start_pos = offset_to_position(source, start);
    let end_pos = offset_to_position(source, end);

    Range {
        start: Position { line: start_pos.0 as u32, character: start_pos.1 as u32 },
        end: Position { line: end_pos.0 as u32, character: end_pos.1 as u32 },
    }
}

/// Convert byte offset to line/column position
#[allow(dead_code)]
fn offset_to_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }

        current_offset += ch.len_utf8();
    }

    (line, column)
}

/// Convert validation errors to LSP diagnostics
#[allow(dead_code)]
pub fn validation_errors_to_diagnostics(
    errors: &[txtx_core::validation::ValidationError],
    _uri: &lsp_types::Url,
) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| {
            let range = Range {
                start: Position {
                    line: error.line.unwrap_or(1).saturating_sub(1) as u32,
                    character: error.column.unwrap_or(0) as u32,
                },
                end: Position {
                    line: error.line.unwrap_or(1).saturating_sub(1) as u32,
                    character: (error.column.unwrap_or(0).saturating_add(10)) as u32, // Approximate end
                },
            };

            Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-validator".to_string()),
                message: error.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position() {
        let source = "line1\nline2\nline3";

        assert_eq!(offset_to_position(source, 0), (0, 0));
        assert_eq!(offset_to_position(source, 5), (0, 5));
        assert_eq!(offset_to_position(source, 6), (1, 0));
        assert_eq!(offset_to_position(source, 12), (2, 0));
    }

    #[test]
    fn test_span_to_range() {
        let source = "line1\nline2\nline3";

        let range = span_to_range(source, 0, 5);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 5);

        let range = span_to_range(source, 6, 11);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 5);
    }
}
