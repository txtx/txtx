//! Environment utility functions shared across LSP handlers
//!
//! Provides common functionality for extracting and working with txtx environments

use lsp_types::Url;
use std::path::Path;

/// Extracts environment name from a file URI.
///
/// Txtx uses dot-separated naming where the **last segment before `.tx`** indicates the environment.
///
/// # Examples
///
/// * `file:///path/config.aws.prod.tx` → `Some("prod")`
/// * `file:///path/signers.devnet.tx` → `Some("devnet")`
/// * `file:///path/some.long.name.with.lots.of.dots.tx` → `Some("dots")`
/// * `file:///path/main.tx` → `None` (no environment specified)
pub fn extract_environment_from_uri(uri: &Url) -> Option<String> {
    uri.to_file_path().ok().and_then(|path| extract_environment_from_path(&path))
}

/// Extracts environment name from a file path.
///
/// Follows txtx naming convention: the **last dot-separated segment before `.tx`** is the environment.
/// If no dots exist before `.tx`, no environment is specified.
///
/// # Examples
///
/// * `config.aws.prod.tx` → `Some("prod")`
/// * `signers.devnet.tx` → `Some("devnet")`
/// * `some.long.name.with.lots.of.dots.tx` → `Some("dots")`
/// * `main.tx` → `None` (no environment specified)
pub fn extract_environment_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let without_ext = file_name.strip_suffix(".tx")?;

    // Extract environment only if filename contains dots (e.g., "config.prod" not "main")
    // The last segment after splitting by dots is the environment name
    without_ext.contains('.').then(|| {
        without_ext.split('.').last().unwrap().to_string()
    })
}

/// Resolves the effective environment for a document.
///
/// Precedence: workspace current environment > URI-inferred environment > global fallback
///
/// This implements txtx's environment resolution strategy across all LSP handlers.
pub fn resolve_environment_for_uri(
    uri: &Url,
    workspace_env: Option<String>,
) -> String {
    workspace_env
        .or_else(|| extract_environment_from_uri(uri))
        .unwrap_or_else(|| "global".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_environment_from_path() {
        // Test environment extraction - last segment before .tx is the environment
        let path = PathBuf::from("/path/to/config.aws.prod.tx");
        assert_eq!(extract_environment_from_path(&path), Some("prod".to_string()));

        let path = PathBuf::from("/path/to/config.dev.tx");
        assert_eq!(extract_environment_from_path(&path), Some("dev".to_string()));

        // Single segment (no dots before .tx) = no environment specified
        let path = PathBuf::from("/path/to/main.tx");
        assert_eq!(extract_environment_from_path(&path), None);

        // Multiple dots - last segment is still the environment
        let path = PathBuf::from("/path/to/some.long.name.with.lots.of.dots.tx");
        assert_eq!(extract_environment_from_path(&path), Some("dots".to_string()));

        // Not a .tx file
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