//! Shared test utilities for LSP tests.
//!
//! Provides helper functions for creating test fixtures like URLs and diagnostics.
//! Reduces code duplication across test modules.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

/// Creates a `file://` URL for testing.
///
/// # Arguments
///
/// * `path` - The file path (without `file:///` prefix)
///
/// # Panics
///
/// Panics if the URL cannot be parsed (should not happen with valid paths).
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::tests::test_utils::url;
/// let uri = url("test.tx");
/// assert_eq!(uri.as_str(), "file:///test.tx");
/// ```
pub fn url(path: &str) -> Url {
    Url::parse(&format!("file:///{}", path)).unwrap()
}

/// Creates an error diagnostic for testing.
///
/// # Arguments
///
/// * `message` - The diagnostic message
/// * `line` - The line number (0-based)
///
/// # Returns
///
/// A diagnostic with ERROR severity spanning columns 0-10 of the given line.
pub fn error_diagnostic(message: &str, line: u32) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, 10)),
        severity: Some(DiagnosticSeverity::ERROR),
        message: message.to_string(),
        ..Default::default()
    }
}

/// Creates a warning diagnostic for testing.
///
/// # Arguments
///
/// * `message` - The diagnostic message
/// * `line` - The line number (0-based)
///
/// # Returns
///
/// A diagnostic with WARNING severity spanning columns 0-10 of the given line.
pub fn warning_diagnostic(message: &str, line: u32) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, 10)),
        severity: Some(DiagnosticSeverity::WARNING),
        message: message.to_string(),
        ..Default::default()
    }
}
