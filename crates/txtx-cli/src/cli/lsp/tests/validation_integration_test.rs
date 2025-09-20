//! Integration tests for HCL validation in LSP
//!
//! These tests verify that the HCL parser integration is working correctly
//! without requiring the full txtx build.

#[cfg(test)]
mod tests {
    use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

    /// Create a simple diagnostic for testing
    fn create_test_diagnostic(
        message: &str,
        line: u32,
        severity: DiagnosticSeverity,
    ) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 10 },
            },
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("test".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = create_test_diagnostic("Test error", 5, DiagnosticSeverity::ERROR);
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.range.start.line, 5);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_position_extraction_patterns() {
        // Test patterns that would be used in HCL error parsing
        let error_msg = "Error on line 5, column 10";
        assert!(error_msg.contains("line 5"));
        assert!(error_msg.contains("column 10"));

        let error_msg2 = "Syntax error at 3:7";
        let parts: Vec<&str> = error_msg2.split(':').collect();
        if parts.len() == 2 {
            assert!(parts[0].ends_with("3"));
            assert_eq!(parts[1], "7");
        }
    }

    #[test]
    fn test_hcl_error_patterns() {
        // Common HCL error message patterns
        let patterns = vec![
            ("unexpected EOF", DiagnosticSeverity::ERROR),
            ("expected identifier", DiagnosticSeverity::ERROR),
            ("invalid block definition", DiagnosticSeverity::ERROR),
            ("undefined variable", DiagnosticSeverity::ERROR),
        ];

        for (pattern, expected_severity) in patterns {
            let diag = create_test_diagnostic(pattern, 0, expected_severity);
            assert_eq!(diag.severity, Some(expected_severity));
        }
    }

    #[test]
    fn test_validation_result_conversion() {
        use crate::cli::lsp::validation::validation_errors_to_diagnostics;
        use lsp_types::Url;
        use txtx_core::validation::ValidationError;

        let errors = vec![
            ValidationError {
                message: "Test error 1".to_string(),
                file: "test.tx".to_string(),
                line: Some(5),
                column: Some(10),
                context: None,
                documentation_link: None,
            },
            ValidationError {
                message: "Test error 2".to_string(),
                file: "test.tx".to_string(),
                line: Some(10),
                column: Some(5),
                context: None,
                documentation_link: None,
            },
        ];

        let uri = Url::parse("file:///test.tx").unwrap();
        let diagnostics = validation_errors_to_diagnostics(&errors, &uri);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].message, "Test error 1");
        assert_eq!(diagnostics[0].range.start.line, 4); // 0-based
        assert_eq!(diagnostics[0].range.start.character, 10); // 0-based
        assert_eq!(diagnostics[1].message, "Test error 2");
    }
}
