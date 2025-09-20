//! Document management for the LSP workspace
//!
//! This module handles the lifecycle of text documents in the LSP workspace,
//! including opening, updating, and closing documents.

use lsp_types::Url;

/// Represents the state of a single document in the workspace
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub content: String,
    pub version: i32,
}

impl Document {
    /// Create a new document
    pub fn new(uri: Url, content: String) -> Self {
        Self { uri, content, version: 1 }
    }

    /// Update the document content and increment version
    pub fn update(&mut self, content: String) {
        self.content = content;
        self.version += 1;
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the current version
    #[allow(dead_code)]
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Check if this is a manifest file (txtx.yml or txtx.yaml)
    pub fn is_manifest(&self) -> bool {
        let path = self.uri.path();
        path.ends_with("txtx.yml")
            || path.ends_with("txtx.yaml")
            || path.ends_with("Txtx.yml")
            || path.ends_with("Txtx.yaml")
    }

    /// Check if this is a runbook file (.tx)
    pub fn is_runbook(&self) -> bool {
        self.uri.path().ends_with(".tx")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let uri = Url::parse("file:///test.tx").unwrap();
        let doc = Document::new(uri.clone(), "content".to_string());

        assert_eq!(doc.uri, uri);
        assert_eq!(doc.content(), "content");
        assert_eq!(doc.version(), 1);
    }

    #[test]
    fn test_document_update() {
        let uri = Url::parse("file:///test.tx").unwrap();
        let mut doc = Document::new(uri, "content".to_string());

        doc.update("new content".to_string());

        assert_eq!(doc.content(), "new content");
        assert_eq!(doc.version(), 2);
    }

    #[test]
    fn test_document_type_detection() {
        let manifest = Document::new(Url::parse("file:///txtx.yml").unwrap(), "".to_string());
        assert!(manifest.is_manifest());
        assert!(!manifest.is_runbook());

        let runbook = Document::new(Url::parse("file:///test.tx").unwrap(), "".to_string());
        assert!(!runbook.is_manifest());
        assert!(runbook.is_runbook());
    }
}
