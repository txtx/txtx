//! State machine for workspace-level state tracking.
//!
//! This module provides explicit state machine infrastructure for tracking and
//! debugging workspace operations. It defines:
//! - [`MachineState`]: Workspace-level states (Ready, Validating, etc.)
//! - [`StateEvent`]: Events that trigger state transitions
//! - [`StateAction`]: Actions to perform after state changes
//! - State transition validation and logging
//!
//! The state machine provides observability and debugging capabilities,
//! complementing the per-document validation state system.

use lsp_types::{Diagnostic, Url};
use std::collections::HashSet;

/// Workspace-level state machine states.
///
/// Tracks the overall workspace state, providing visibility into what the
/// LSP server is currently doing. This is separate from per-document
/// [`ValidationStatus`](super::ValidationStatus) which tracks individual
/// document states.
///
/// # State Diagram
///
/// ```text
/// Uninitialized -> Indexing -> Ready
///                        ↓         ↑
///                  IndexingError   |
///                        ↓         |
///                    Indexing -----+
///
/// Ready -> Validating -> Ready
///   ↓         ↓           ↑
///   ↓    ValidationError  |
///   ↓         ↓           |
///   ↓      Validating ----+
///   ↓
///   +-> EnvironmentChanging -> Revalidating -> Ready
///   ↓
///   +-> DependencyResolving -> Invalidating -> Revalidating -> Ready
/// ```
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::state_machine::MachineState;
/// let state = MachineState::Ready;
/// assert!(state.can_accept_requests());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineState {
    /// Initial state before LSP initialization.
    Uninitialized,

    /// Indexing workspace files (manifests and runbooks).
    Indexing,

    /// Failed to index workspace (parse errors, etc.).
    IndexingError,

    /// Ready to accept requests and process changes.
    Ready,

    /// Validating a single document.
    ///
    /// # Fields
    ///
    /// * `document` - URI of the document being validated
    Validating { document: Url },

    /// Switching to a new environment.
    ///
    /// # Fields
    ///
    /// * `new_env` - Name of the new environment being switched to
    EnvironmentChanging { new_env: String },

    /// Re-validating multiple documents.
    ///
    /// # Fields
    ///
    /// * `documents` - List of documents to re-validate
    /// * `current` - Index of the document currently being validated
    Revalidating {
        documents: Vec<Url>,
        current: usize,
    },

    /// Resolving dependencies after document changes.
    DependencyResolving,

    /// Invalidating documents affected by changes.
    ///
    /// # Fields
    ///
    /// * `affected` - Set of document URIs that need re-validation
    Invalidating { affected: HashSet<Url> },
}

impl MachineState {
    /// Returns `true` if the workspace can accept new requests.
    ///
    /// Only [`Ready`](Self::Ready) state accepts requests.
    pub fn can_accept_requests(&self) -> bool {
        matches!(self, MachineState::Ready)
    }

    /// Returns `true` if validation is in progress.
    pub fn is_validating(&self) -> bool {
        matches!(
            self,
            MachineState::Validating { .. } | MachineState::Revalidating { .. }
        )
    }

    /// Returns a human-readable description of the current state.
    ///
    /// Includes relevant details like document URIs and environment names
    /// for logging and debugging.
    ///
    /// # Examples
    ///
    /// ```
    /// # use txtx_cli::cli::lsp::workspace::state_machine::MachineState;
    /// # use lsp_types::Url;
    /// let uri = Url::parse("file:///test.tx").unwrap();
    /// let state = MachineState::Validating { document: uri };
    /// assert!(state.description().contains("Validating"));
    /// assert!(state.description().contains("test.tx"));
    /// ```
    pub fn description(&self) -> String {
        match self {
            MachineState::Uninitialized => "Uninitialized".to_string(),
            MachineState::Indexing => "Indexing workspace".to_string(),
            MachineState::IndexingError => "Indexing error".to_string(),
            MachineState::Ready => "Ready".to_string(),
            MachineState::Validating { document } => {
                format!("Validating document: {}", document.path())
            }
            MachineState::EnvironmentChanging { new_env } => {
                format!("Switching to environment: {}", new_env)
            }
            MachineState::Revalidating { documents, current } => {
                format!("Revalidating {} documents (at {})", documents.len(), current)
            }
            MachineState::DependencyResolving => "Resolving dependencies".to_string(),
            MachineState::Invalidating { affected } => {
                format!("Invalidating {} documents", affected.len())
            }
        }
    }
}

impl Default for MachineState {
    fn default() -> Self {
        MachineState::Uninitialized
    }
}

/// Events that trigger state machine transitions.
///
/// These events represent all the ways the workspace state can change.
/// Processing an event through [`WorkspaceState::process_event`] produces
/// a new [`MachineState`] and potentially some [`StateAction`]s to perform.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::state_machine::StateEvent;
/// # use lsp_types::Url;
/// let uri = Url::parse("file:///test.tx").unwrap();
/// let event = StateEvent::DocumentOpened {
///     uri: uri.clone(),
///     content: "action \"test\" {}".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub enum StateEvent {
    /// LSP server initialized, starting workspace indexing.
    Initialize,

    /// Workspace indexing completed successfully.
    IndexingComplete,

    /// Workspace indexing failed.
    ///
    /// # Fields
    ///
    /// * `error` - Description of the error
    IndexingFailed { error: String },

    /// Document opened in editor.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the opened document
    /// * `content` - Initial content of the document
    DocumentOpened { uri: Url, content: String },

    /// Document content changed.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the changed document
    /// * `content` - New content of the document
    DocumentChanged { uri: Url, content: String },

    /// Document closed in editor.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the closed document
    DocumentClosed { uri: Url },

    /// User switched to a different environment.
    ///
    /// # Fields
    ///
    /// * `new_env` - Name of the new environment
    EnvironmentChanged { new_env: String },

    /// Validation completed for a document.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the validated document
    /// * `diagnostics` - Diagnostics produced by validation
    /// * `success` - Whether validation completed without errors
    ValidationCompleted {
        uri: Url,
        diagnostics: Vec<Diagnostic>,
        success: bool,
    },

    /// Dependency graph changed, affecting other documents.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the document whose dependencies changed
    /// * `affected` - Set of documents affected by the change
    DependencyChanged { uri: Url, affected: HashSet<Url> },
}

/// Actions to perform after state transitions.
///
/// When processing a [`StateEvent`], the state machine may produce actions
/// that the LSP server should perform. Actions represent side effects like
/// validating documents or publishing diagnostics.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::state_machine::StateAction;
/// # use lsp_types::Url;
/// let uri = Url::parse("file:///test.tx").unwrap();
/// let action = StateAction::ValidateDocument { uri };
/// ```
#[derive(Debug, Clone)]
pub enum StateAction {
    /// Validate a specific document.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the document to validate
    ValidateDocument { uri: Url },

    /// Publish diagnostics to the editor.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the document
    /// * `diagnostics` - Diagnostics to publish
    PublishDiagnostics {
        uri: Url,
        diagnostics: Vec<Diagnostic>,
    },

    /// Invalidate validation cache for a document.
    ///
    /// # Fields
    ///
    /// * `uri` - URI of the document
    InvalidateCache { uri: Url },

    /// Refresh dependency graph by re-extracting dependencies.
    RefreshDependencies,

    /// Log a state transition for debugging.
    ///
    /// # Fields
    ///
    /// * `message` - Log message describing the transition
    LogTransition { message: String },
}

/// State transition tracking for debugging and observability.
///
/// Records state transitions with timestamps to provide an audit trail.
/// Useful for debugging complex validation scenarios and understanding
/// the sequence of events that led to a particular state.
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// State before the transition.
    pub from: MachineState,
    /// State after the transition.
    pub to: MachineState,
    /// Event that triggered the transition.
    pub event: String,
    /// Timestamp of the transition.
    pub timestamp: std::time::SystemTime,
}

impl StateTransition {
    /// Creates a new state transition record with the current timestamp.
    pub fn new(from: MachineState, to: MachineState, event: impl Into<String>) -> Self {
        Self {
            from,
            to,
            event: event.into(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Returns a human-readable representation.
    ///
    /// Format: `"Ready -> Validating (DocumentChanged)"`
    pub fn format(&self) -> String {
        format!(
            "{} -> {} ({})",
            self.from.description(),
            self.to.description(),
            self.event
        )
    }
}

/// State machine history for debugging.
///
/// Maintains a bounded history of state transitions. Useful for diagnosing
/// issues by reconstructing the sequence of events that led to the current state.
#[derive(Debug, Clone)]
pub struct StateHistory {
    /// Recent transitions (bounded to prevent unbounded memory growth).
    transitions: Vec<StateTransition>,
    /// Maximum number of transitions to keep.
    max_size: usize,
}

impl StateHistory {
    /// Creates a new state history with bounded capacity.
    pub fn new(max_size: usize) -> Self {
        Self {
            transitions: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Records a state transition, removing oldest if at capacity.
    pub fn record(&mut self, transition: StateTransition) {
        if self.transitions.len() >= self.max_size {
            self.transitions.remove(0);
        }
        self.transitions.push(transition);
    }

    /// Returns all recorded transitions in chronological order.
    pub fn transitions(&self) -> &[StateTransition] {
        &self.transitions
    }

    /// Clears all recorded transitions.
    pub fn clear(&mut self) {
        self.transitions.clear();
    }

    /// Returns a multi-line formatted history for logging.
    pub fn format(&self) -> String {
        self.transitions
            .iter()
            .map(|t| t.format())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for StateHistory {
    fn default() -> Self {
        Self::new(50) // Keep last 50 transitions by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_state_default() {
        assert_eq!(MachineState::default(), MachineState::Uninitialized);
    }

    #[test]
    fn test_can_accept_requests() {
        assert!(MachineState::Ready.can_accept_requests());
        assert!(!MachineState::Uninitialized.can_accept_requests());
        assert!(!MachineState::Indexing.can_accept_requests());
    }

    #[test]
    fn test_is_validating() {
        let uri = Url::parse("file:///test.tx").unwrap();

        assert!(MachineState::Validating {
            document: uri.clone()
        }
        .is_validating());
        assert!(MachineState::Revalidating {
            documents: vec![uri],
            current: 0
        }
        .is_validating());
        assert!(!MachineState::Ready.is_validating());
    }

    #[test]
    fn test_description() {
        assert_eq!(MachineState::Ready.description(), "Ready");
        assert_eq!(MachineState::Indexing.description(), "Indexing workspace");

        let uri = Url::parse("file:///test.tx").unwrap();
        let desc = MachineState::Validating {
            document: uri.clone(),
        }
        .description();
        assert!(desc.contains("Validating"));
        assert!(desc.contains("test.tx"));
    }

    #[test]
    fn test_state_transition_format() {
        let from = MachineState::Ready;
        let to = MachineState::Indexing;
        let transition = StateTransition::new(from, to, "Initialize");

        let formatted = transition.format();
        assert!(formatted.contains("Ready"));
        assert!(formatted.contains("Indexing"));
        assert!(formatted.contains("Initialize"));
    }

    #[test]
    fn test_state_history_bounds() {
        let mut history = StateHistory::new(3);

        // Add 5 transitions
        for i in 0..5 {
            history.record(StateTransition::new(
                MachineState::Ready,
                MachineState::Indexing,
                format!("Event {}", i),
            ));
        }

        // Should only keep last 3
        assert_eq!(history.transitions().len(), 3);
        assert_eq!(history.transitions()[0].event, "Event 2");
        assert_eq!(history.transitions()[2].event, "Event 4");
    }

    #[test]
    fn test_state_history_clear() {
        let mut history = StateHistory::new(10);
        history.record(StateTransition::new(
            MachineState::Ready,
            MachineState::Indexing,
            "Test",
        ));

        assert_eq!(history.transitions().len(), 1);
        history.clear();
        assert_eq!(history.transitions().len(), 0);
    }
}
