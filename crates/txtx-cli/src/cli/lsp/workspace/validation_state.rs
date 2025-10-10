//! Validation state tracking for LSP documents.
//!
//! This module provides the [`ValidationState`] type for tracking validation status,
//! caching diagnostics, and detecting when re-validation is needed based on content
//! or environment changes.

use lsp_types::{Diagnostic, Url};
use std::collections::HashSet;
use std::time::SystemTime;

/// Per-document validation state.
///
/// Tracks validation results and metadata to determine when a document needs
/// re-validation. Uses content hashing and environment tracking to avoid
/// redundant validation operations.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::ValidationState;
/// # use txtx_cli::cli::lsp::workspace::ValidationStatus;
/// let mut state = ValidationState::new();
/// assert_eq!(state.status, ValidationStatus::Unvalidated);
///
/// state.update_with_results(
///     ValidationStatus::Clean,
///     12345,
///     Some("production".to_string()),
///     vec![],
/// );
/// assert!(state.is_valid_for(12345, &Some("production".to_string())));
/// ```
#[derive(Debug, Clone)]
pub struct ValidationState {
    /// Current validation status.
    pub status: ValidationStatus,
    /// Last validation timestamp.
    pub last_validated: SystemTime,
    /// Content hash when last validated.
    pub content_hash: u64,
    /// Environment used for validation.
    pub validated_environment: Option<String>,
    /// Cached diagnostics from the last validation.
    pub diagnostics: Vec<Diagnostic>,
    /// Dependencies that affect this document.
    pub dependencies: HashSet<Url>,
}

impl ValidationState {
    /// Creates a new unvalidated state.
    ///
    /// The initial state has:
    /// - Status: [`ValidationStatus::Unvalidated`]
    /// - Content hash: 0
    /// - No validated environment
    /// - Empty diagnostics
    /// - No dependencies
    pub fn new() -> Self {
        Self {
            status: ValidationStatus::Unvalidated,
            last_validated: SystemTime::now(),
            content_hash: 0,
            validated_environment: None,
            diagnostics: Vec::new(),
            dependencies: HashSet::new(),
        }
    }

    /// Updates validation state with new results.
    ///
    /// # Arguments
    ///
    /// * `status` - The new validation status
    /// * `content_hash` - Hash of the content that was validated
    /// * `environment` - Environment name used during validation
    /// * `diagnostics` - Diagnostics produced by validation
    pub fn update_with_results(
        &mut self,
        status: ValidationStatus,
        content_hash: u64,
        environment: Option<String>,
        diagnostics: Vec<Diagnostic>,
    ) {
        self.status = status;
        self.last_validated = SystemTime::now();
        self.content_hash = content_hash;
        self.validated_environment = environment;
        self.diagnostics = diagnostics;
    }

    /// Marks this validation as stale (needs re-validation).
    ///
    /// This is called when a dependency changes, requiring re-validation
    /// even if the document's content hasn't changed. Does nothing if the
    /// document is already unvalidated.
    pub fn mark_stale(&mut self) {
        if self.status != ValidationStatus::Unvalidated {
            self.status = ValidationStatus::Stale;
        }
    }

    /// Checks if this state is valid for the current context.
    ///
    /// Returns `true` only if:
    /// - The content hash matches (content hasn't changed)
    /// - The environment matches (environment hasn't switched)
    /// - The status indicates validation is complete and not stale
    ///
    /// # Arguments
    ///
    /// * `content_hash` - Current hash of the document content
    /// * `environment` - Current environment selection
    ///
    /// # Returns
    ///
    /// `true` if cached validation is still valid, `false` if re-validation is needed.
    pub fn is_valid_for(&self, content_hash: u64, environment: &Option<String>) -> bool {
        // Not valid if content changed
        if self.content_hash != content_hash {
            return false;
        }

        // Not valid if environment changed
        if &self.validated_environment != environment {
            return false;
        }

        // Not valid if marked as stale or unvalidated
        self.status.is_validated()
    }
}

impl Default for ValidationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation status for a document.
///
/// Tracks the lifecycle of document validation from initial state through
/// validation completion, including error states and staleness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Never validated.
    Unvalidated,
    /// Currently validating.
    Validating,
    /// Validated with no errors or warnings.
    Clean,
    /// Validated with warnings only.
    Warning,
    /// Validated with errors.
    Error,
    /// Needs re-validation (dependency or environment changed).
    Stale,
    /// Cyclic dependency detected.
    CyclicDependency,
}

impl ValidationStatus {
    /// Checks if this status indicates the document has been validated.
    ///
    /// Returns `true` for [`Clean`](Self::Clean), [`Warning`](Self::Warning),
    /// [`Error`](Self::Error), and [`CyclicDependency`](Self::CyclicDependency).
    /// Returns `false` for [`Unvalidated`](Self::Unvalidated),
    /// [`Validating`](Self::Validating), and [`Stale`](Self::Stale).
    pub fn is_validated(&self) -> bool {
        matches!(
            self,
            ValidationStatus::Clean
                | ValidationStatus::Warning
                | ValidationStatus::Error
                | ValidationStatus::CyclicDependency
        )
    }

    /// Checks if this status indicates errors.
    ///
    /// Returns `true` for [`Error`](Self::Error) and
    /// [`CyclicDependency`](Self::CyclicDependency).
    pub fn has_errors(&self) -> bool {
        matches!(self, ValidationStatus::Error | ValidationStatus::CyclicDependency)
    }

    /// Determines status from LSP diagnostics.
    ///
    /// Returns:
    /// - [`Clean`](Self::Clean) if diagnostics is empty
    /// - [`Error`](Self::Error) if any diagnostic has ERROR severity
    /// - [`Warning`](Self::Warning) if diagnostics only contain warnings
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - Slice of LSP diagnostics to analyze
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        use lsp_types::DiagnosticSeverity;

        if diagnostics.is_empty() {
            return ValidationStatus::Clean;
        }

        let has_errors = diagnostics.iter().any(|d| {
            d.severity == Some(DiagnosticSeverity::ERROR)
        });

        if has_errors {
            ValidationStatus::Error
        } else {
            ValidationStatus::Warning
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_state_new() {
        let state = ValidationState::new();
        assert_eq!(state.status, ValidationStatus::Unvalidated);
        assert_eq!(state.content_hash, 0);
        assert!(state.diagnostics.is_empty());
    }

    #[test]
    fn test_mark_stale() {
        let mut state = ValidationState::new();
        state.status = ValidationStatus::Clean;

        state.mark_stale();
        assert_eq!(state.status, ValidationStatus::Stale);
    }

    #[test]
    fn test_is_valid_for() {
        let mut state = ValidationState::new();
        state.status = ValidationStatus::Clean;
        state.content_hash = 12345;
        state.validated_environment = Some("sepolia".to_string());

        // Valid for same content and environment
        assert!(state.is_valid_for(12345, &Some("sepolia".to_string())));

        // Invalid for different content
        assert!(!state.is_valid_for(54321, &Some("sepolia".to_string())));

        // Invalid for different environment
        assert!(!state.is_valid_for(12345, &Some("mainnet".to_string())));

        // Invalid if stale
        state.mark_stale();
        assert!(!state.is_valid_for(12345, &Some("sepolia".to_string())));
    }

    #[test]
    fn test_status_from_diagnostics() {
        use lsp_types::{DiagnosticSeverity, Position, Range};

        // Empty diagnostics = Clean
        assert_eq!(ValidationStatus::from_diagnostics(&[]), ValidationStatus::Clean);

        // Warnings only = Warning
        let warnings = vec![Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::WARNING),
            message: "warning".to_string(),
            ..Default::default()
        }];
        assert_eq!(ValidationStatus::from_diagnostics(&warnings), ValidationStatus::Warning);

        // Errors = Error
        let errors = vec![Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::ERROR),
            message: "error".to_string(),
            ..Default::default()
        }];
        assert_eq!(ValidationStatus::from_diagnostics(&errors), ValidationStatus::Error);
    }
}
