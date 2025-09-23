//! Tests for HCL diagnostic integration

#[cfg(test)]
mod tests {
    use crate::cli::lsp::diagnostics_hcl_integrated::validate_runbook_with_hcl;
    use lsp_types::Url;

    #[test]
    fn test_hcl_syntax_error_detection() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Test with invalid HCL syntax
        let content = r#"
addon "evm" {
    chain_id = 1
    # Missing closing brace
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        assert!(!diagnostics.is_empty(), "Should detect syntax error");

        let first_diag = &diagnostics[0];
        assert!(first_diag.message.contains("parse error") || first_diag.message.contains("HCL"));
        assert_eq!(first_diag.source.as_deref(), Some("hcl-parser"));
    }

    #[test]
    fn test_valid_hcl_with_semantic_errors() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Valid HCL but with semantic errors
        let content = r#"
action "deploy" "unknown::action" {
    signer = "undefined_signer"
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        // Should have errors for unknown namespace and undefined signer
        assert!(diagnostics.len() >= 1, "Should detect semantic errors");
    }

    #[test]
    fn test_clean_runbook() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Valid runbook with no errors
        let content = r#"
addon "evm" "ethereum" {
    chain_id = 1
}

variable "contract_address" {
    value = "0x123"
}

output "result" {
    value = variable.contract_address
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        // Should have no errors for a clean runbook
        assert!(
            diagnostics.is_empty()
                || diagnostics
                    .iter()
                    .all(|d| d.severity != Some(lsp_types::DiagnosticSeverity::ERROR)),
            "Should have no errors for valid runbook"
        );
    }

    #[test]
    fn test_hcl_error_position_extraction() {
        use crate::cli::lsp::diagnostics_hcl_integrated::extract_position_from_error;

        // Test various error message formats
        assert_eq!(extract_position_from_error("Error on line 5, column 10"), (5, 10));
        assert_eq!(extract_position_from_error("Syntax error at 3:7"), (3, 7));
        assert_eq!(extract_position_from_error("Parse failed on line 2"), (2, 1));
        assert_eq!(extract_position_from_error("Unknown error"), (1, 1));
    }
}
