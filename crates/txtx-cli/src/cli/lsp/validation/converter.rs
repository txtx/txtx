//! Convert doctor validation results to LSP diagnostics

use crate::cli::doctor::ValidationOutcome;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use txtx_core::validation::ValidationSuggestion;

/// Convert a doctor ValidationOutcome to an optional LSP Diagnostic
pub fn validation_outcome_to_diagnostic(
    outcome: ValidationOutcome,
    range: Range,
) -> Option<Diagnostic> {
    match outcome {
        ValidationOutcome::Pass => None,
        ValidationOutcome::Error { message, context, suggestion, documentation_link } => {
            let mut diagnostic_message = message;
            if let Some(ctx) = context {
                diagnostic_message.push_str(&format!("\n\nContext: {}", ctx));
            }
            if let Some(sug) = suggestion {
                diagnostic_message
                    .push_str(&format!("\n\nSuggestion: {}", format_suggestion(&sug)));
            }
            if let Some(link) = documentation_link {
                diagnostic_message.push_str(&format!("\n\nSee: {}", link));
            }

            Some(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-doctor".to_string()),
                message: diagnostic_message,
                related_information: None,
                tags: None,
                data: None,
            })
        }
        ValidationOutcome::Warning { message, suggestion } => {
            let mut diagnostic_message = message;
            if let Some(sug) = suggestion {
                diagnostic_message
                    .push_str(&format!("\n\nSuggestion: {}", format_suggestion(&sug)));
            }

            Some(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("txtx-doctor".to_string()),
                message: diagnostic_message,
                related_information: None,
                tags: None,
                data: None,
            })
        }
    }
}

/// Format a ValidationSuggestion for display
fn format_suggestion(suggestion: &ValidationSuggestion) -> String {
    if let Some(example) = &suggestion.example {
        format!("{}\nExample: {}", suggestion.message, example)
    } else {
        suggestion.message.clone()
    }
}

/// Convert a location string to an LSP Range
#[allow(dead_code)]
fn location_to_range(location: &str) -> Range {
    // Parse location format "line:col" or "line:col-endcol"
    let parts: Vec<&str> = location.split(':').collect();

    if parts.len() >= 2 {
        let line = parts[0].parse::<u32>().unwrap_or(0);
        let col_parts: Vec<&str> = parts[1].split('-').collect();
        let start_col = col_parts[0].parse::<u32>().unwrap_or(0);
        let end_col = col_parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(start_col + 1);

        Range {
            start: Position {
                line: line.saturating_sub(1),
                character: start_col.saturating_sub(1),
            },
            end: Position { line: line.saturating_sub(1), character: end_col.saturating_sub(1) },
        }
    } else {
        // Default to first character of first line
        Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 1 } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_outcome_to_diagnostic() {
        let outcome = ValidationOutcome::Error {
            message: "Missing required input".to_string(),
            context: Some("In action 'deploy'".to_string()),
            suggestion: Some(ValidationSuggestion {
                message: "Add: input = \"value\"".to_string(),
                example: Some("input = \"value\"".to_string()),
            }),
            documentation_link: Some("https://docs.txtx.sh/inputs".to_string()),
        };

        let range = Range {
            start: Position { line: 9, character: 4 },
            end: Position { line: 9, character: 10 },
        };

        let diagnostic = validation_outcome_to_diagnostic(outcome, range).unwrap();

        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diagnostic.message.contains("Missing required input"));
        assert!(diagnostic.message.contains("Context: In action 'deploy'"));
        assert!(diagnostic.message.contains("Suggestion: Add: input = \"value\""));
        assert!(diagnostic.message.contains("https://docs.txtx.sh/inputs"));
    }

    #[test]
    fn test_warning_outcome_to_diagnostic() {
        let outcome = ValidationOutcome::Warning {
            message: "Input may be undefined".to_string(),
            suggestion: Some(ValidationSuggestion {
                message: "Consider providing a default value".to_string(),
                example: None,
            }),
        };

        let range = Range {
            start: Position { line: 5, character: 0 },
            end: Position { line: 5, character: 10 },
        };

        let diagnostic = validation_outcome_to_diagnostic(outcome, range).unwrap();

        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
        assert!(diagnostic.message.contains("Input may be undefined"));
        assert!(diagnostic.message.contains("Consider providing a default value"));
    }

    #[test]
    fn test_pass_outcome_returns_none() {
        let outcome = ValidationOutcome::Pass;
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 1 },
        };

        assert!(validation_outcome_to_diagnostic(outcome, range).is_none());
    }

    #[test]
    fn test_location_parsing() {
        let range = location_to_range("5:10-15");
        assert_eq!(range.start.line, 4);
        assert_eq!(range.start.character, 9);
        assert_eq!(range.end.line, 4);
        assert_eq!(range.end.character, 14);
    }
}
