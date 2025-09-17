//! Adapter to integrate doctor validation into LSP diagnostics

use super::converter::validation_outcome_to_diagnostic;
use crate::cli::doctor::{
    CliInputOverrideRule, InputDefinedRule, InputNamingConventionRule, SensitiveDataRule,
    ValidationContext, ValidationRule,
};
use lsp_types::{Diagnostic, Position, Range, Url};
use std::collections::HashMap;
use txtx_core::manifest::WorkspaceManifest;

/// Adapter that runs doctor validation rules and produces LSP diagnostics
pub struct DoctorValidationAdapter {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl DoctorValidationAdapter {
    /// Create a new adapter with default validation rules
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(InputDefinedRule),
                Box::new(InputNamingConventionRule),
                Box::new(CliInputOverrideRule),
                Box::new(SensitiveDataRule),
            ],
        }
    }

    /// Run validation on a document and return diagnostics
    pub fn validate_document(
        &self,
        uri: &Url,
        content: &str,
        manifest: Option<&WorkspaceManifest>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // If we don't have a manifest, we can't run most validations
        let Some(manifest) = manifest else {
            return diagnostics;
        };

        // Extract file path from URI
        let file_path = uri.path();

        // For now, we'll create a simple validation context
        // In a real implementation, this would parse the document to find inputs
        let context = ValidationContext {
            input_name: "example",
            full_name: "input.example",
            manifest,
            environment: None,
            effective_inputs: &HashMap::new(),
            cli_inputs: &[],
            content,
            file_path,
        };

        // Run each validation rule
        for rule in &self.rules {
            let outcome = rule.check(&context);

            // For now, use a default range (whole first line)
            // In a real implementation, we'd parse locations from the content
            let range = Range {
                start: Position { line: 0, character: 0 },
                end: Position {
                    line: 0,
                    character: content.lines().next().map(|l| l.len()).unwrap_or(0) as u32,
                },
            };

            if let Some(diagnostic) = validation_outcome_to_diagnostic(outcome, range) {
                diagnostics.push(diagnostic);
            }
        }

        diagnostics
    }

    /// Add a custom validation rule
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }

    /// Clear all validation rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }
}

impl Default for DoctorValidationAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = DoctorValidationAdapter::new();
        assert_eq!(adapter.rules.len(), 4); // We have 4 default rules
    }

    #[test]
    fn test_validation_without_manifest() {
        let adapter = DoctorValidationAdapter::new();
        let uri = Url::parse("file:///test.tx").unwrap();
        let content = "test content";

        let diagnostics = adapter.validate_document(&uri, content, None);
        assert!(diagnostics.is_empty()); // No diagnostics without manifest
    }

    // More comprehensive tests would require mocking WorkspaceManifest
}
