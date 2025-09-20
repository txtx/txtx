//! Environment utility functions shared across LSP handlers
//!
//! Provides common functionality for extracting and working with txtx environments

use lsp_types::Url;
use std::path::Path;

/// Extract environment from a file URI
/// For "config.aws.prod.tx", returns Some("prod")
/// For "main.tx", returns None
pub fn extract_environment_from_uri(uri: &Url) -> Option<String> {
    if let Ok(path) = uri.to_file_path() {
        extract_environment_from_path(&path)
    } else {
        None
    }
}

/// Extract environment from a file path
/// For "config.aws.prod.tx", returns Some("prod")
/// For "main.tx", returns None
pub fn extract_environment_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    
    // Must end with .tx
    if !file_name.ends_with(".tx") {
        return None;
    }
    
    // Remove .tx extension
    let without_ext = &file_name[..file_name.len() - 3];
    
    // Split by dots
    let parts: Vec<&str> = without_ext.split('.').collect();
    
    // If there are at least 2 parts, the last one is the environment
    if parts.len() >= 2 {
        Some(parts[parts.len() - 1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_environment_from_path() {
        // Test environment extraction
        let path = PathBuf::from("/path/to/config.aws.prod.tx");
        assert_eq!(extract_environment_from_path(&path), Some("prod".to_string()));

        let path = PathBuf::from("/path/to/config.dev.tx");
        assert_eq!(extract_environment_from_path(&path), Some("dev".to_string()));

        let path = PathBuf::from("/path/to/main.tx");
        assert_eq!(extract_environment_from_path(&path), None);

        let path = PathBuf::from("/path/to/config.txt");
        assert_eq!(extract_environment_from_path(&path), None);
    }

    #[test]
    fn test_extract_environment_from_uri() {
        let uri = Url::parse("file:///path/to/config.aws.prod.tx").unwrap();
        assert_eq!(extract_environment_from_uri(&uri), Some("prod".to_string()));

        let uri = Url::parse("file:///path/to/main.tx").unwrap();
        assert_eq!(extract_environment_from_uri(&uri), None);
    }
}