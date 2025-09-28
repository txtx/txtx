//! Tests for the workspace-level state machine (Phase 6).
//!
//! This test suite verifies the state machine infrastructure provides:
//! - Correct state transitions for all events
//! - State history tracking
//! - Action generation
//! - Integration with existing validation flow

use crate::cli::lsp::workspace::{
    MachineState, SharedWorkspaceState, StateAction, StateEvent, WorkspaceState,
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

/// Helper to create a test URL.
fn url(path: &str) -> Url {
    Url::parse(&format!("file:///{}", path)).unwrap()
}

/// Helper to create a simple diagnostic.
fn diagnostic(message: &str, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 1)),
        severity: Some(severity),
        message: message.to_string(),
        ..Default::default()
    }
}

#[test]
fn test_initial_state() {
    let workspace = WorkspaceState::new();
    assert_eq!(*workspace.get_machine_state(), MachineState::Uninitialized);
}

#[test]
fn test_initialize_transition() {
    let mut workspace = WorkspaceState::new();

    let actions = workspace.process_event(StateEvent::Initialize);
    assert_eq!(*workspace.get_machine_state(), MachineState::Indexing);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        StateAction::LogTransition { .. }
    ));
}

#[test]
fn test_indexing_complete_transition() {
    let mut workspace = WorkspaceState::new();

    // First initialize
    workspace.process_event(StateEvent::Initialize);
    assert_eq!(*workspace.get_machine_state(), MachineState::Indexing);

    // Then complete indexing
    let actions = workspace.process_event(StateEvent::IndexingComplete);
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        StateAction::LogTransition { .. }
    ));
}

#[test]
fn test_indexing_failed_transition() {
    let mut workspace = WorkspaceState::new();

    workspace.process_event(StateEvent::Initialize);
    let actions = workspace.process_event(StateEvent::IndexingFailed {
        error: "Parse error".to_string(),
    });

    assert_eq!(
        *workspace.get_machine_state(),
        MachineState::IndexingError
    );
    assert!(matches!(
        actions[0],
        StateAction::LogTransition { .. }
    ));
}

#[test]
fn test_document_opened_transition() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("test.tx");
    let actions = workspace.process_event(StateEvent::DocumentOpened {
        uri: uri.clone(),
        content: "action \"test\" {}".to_string(),
    });

    assert_eq!(
        *workspace.get_machine_state(),
        MachineState::Validating {
            document: uri.clone()
        }
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        StateAction::ValidateDocument { .. }
    ));
}

#[test]
fn test_document_changed_transition() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("test.tx");
    let actions = workspace.process_event(StateEvent::DocumentChanged {
        uri: uri.clone(),
        content: "action \"test\" { description = \"updated\" }".to_string(),
    });

    assert_eq!(
        *workspace.get_machine_state(),
        MachineState::Validating {
            document: uri.clone()
        }
    );
    assert_eq!(actions.len(), 1);
}

#[test]
fn test_validation_completed_single_document() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("test.tx");
    workspace.process_event(StateEvent::DocumentOpened {
        uri: uri.clone(),
        content: "action \"test\" {}".to_string(),
    });

    // Complete validation
    let diagnostics = vec![];
    let actions = workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri.clone(),
        diagnostics: diagnostics.clone(),
        success: true,
    });

    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        StateAction::PublishDiagnostics { uri: u, diagnostics: d }
        if u == &uri && d.is_empty()
    ));
}

#[test]
fn test_environment_changed_transition() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    // Open a runbook
    let uri = url("test.tx");
    workspace.open_document(uri.clone(), "action \"test\" {}".to_string());

    // Change environment
    let actions = workspace.process_event(StateEvent::EnvironmentChanged {
        new_env: "production".to_string(),
    });

    // Should transition through EnvironmentChanging -> Revalidating
    assert!(matches!(
        *workspace.get_machine_state(),
        MachineState::Revalidating { .. }
    ));

    // Should generate validation actions for the runbook
    assert!(!actions.is_empty());
    assert!(actions
        .iter()
        .any(|a| matches!(a, StateAction::ValidateDocument { .. })));
}

#[test]
fn test_environment_changed_no_runbooks() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    // Change environment with no runbooks open
    let actions = workspace.process_event(StateEvent::EnvironmentChanged {
        new_env: "production".to_string(),
    });

    // Should go straight to Ready since there are no runbooks to revalidate
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
}

#[test]
fn test_revalidating_multiple_documents() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    // Open multiple runbooks
    let uri1 = url("test1.tx");
    let uri2 = url("test2.tx");
    workspace.open_document(uri1.clone(), "action \"test1\" {}".to_string());
    workspace.open_document(uri2.clone(), "action \"test2\" {}".to_string());

    // Change environment to trigger revalidation
    workspace.process_event(StateEvent::EnvironmentChanged {
        new_env: "production".to_string(),
    });

    // Should be in Revalidating state
    match workspace.get_machine_state() {
        MachineState::Revalidating { documents, current } => {
            assert_eq!(documents.len(), 2);
            assert_eq!(*current, 0);
        }
        _ => panic!("Expected Revalidating state"),
    }

    // Complete first validation
    workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri1.clone(),
        diagnostics: vec![],
        success: true,
    });

    // Should still be revalidating
    match workspace.get_machine_state() {
        MachineState::Revalidating { documents, current } => {
            assert_eq!(documents.len(), 2);
            assert_eq!(*current, 1);
        }
        _ => panic!("Expected Revalidating state"),
    }

    // Complete second validation
    workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri2.clone(),
        diagnostics: vec![],
        success: true,
    });

    // Should be back to Ready
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
}

#[test]
fn test_dependency_changed_transition() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    // Open documents
    let manifest_uri = url("txtx.yml");
    let runbook_uri = url("deploy.tx");
    workspace.open_document(manifest_uri.clone(), "environments:".to_string());
    workspace.open_document(runbook_uri.clone(), "action \"test\" {}".to_string());

    // Simulate dependency change
    let mut affected = std::collections::HashSet::new();
    affected.insert(runbook_uri.clone());

    let actions = workspace.process_event(StateEvent::DependencyChanged {
        uri: manifest_uri.clone(),
        affected: affected.clone(),
    });

    // Should transition through Invalidating -> Revalidating
    assert!(matches!(
        *workspace.get_machine_state(),
        MachineState::Revalidating { .. }
    ));

    // Should generate actions to invalidate and validate affected documents
    assert!(!actions.is_empty());
    assert!(actions
        .iter()
        .any(|a| matches!(a, StateAction::InvalidateCache { uri } if uri == &runbook_uri)));
    assert!(actions
        .iter()
        .any(|a| matches!(a, StateAction::ValidateDocument { uri } if uri == &runbook_uri)));
}

#[test]
fn test_dependency_changed_no_affected() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("txtx.yml");
    let affected = std::collections::HashSet::new();

    workspace.process_event(StateEvent::DependencyChanged {
        uri,
        affected,
    });

    // Should go straight to Ready with no affected documents
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
}

#[test]
fn test_document_closed_no_state_change() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("test.tx");
    let actions = workspace.process_event(StateEvent::DocumentClosed { uri: uri.clone() });

    // Document closing shouldn't change the Ready state
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);

    // Should generate cache invalidation action
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        StateAction::InvalidateCache { .. }
    ));
}

#[test]
fn test_state_history_recording() {
    let mut workspace = WorkspaceState::new();

    // Perform several state transitions
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    let uri = url("test.tx");
    workspace.process_event(StateEvent::DocumentOpened {
        uri: uri.clone(),
        content: "".to_string(),
    });

    // Check history was recorded
    let history = workspace.get_state_history();
    assert!(history.transitions().len() >= 3);

    // Verify transition order
    let transitions = history.transitions();
    assert!(transitions[0]
        .format()
        .contains("Uninitialized -> Indexing"));
    assert!(transitions[1].format().contains("Indexing workspace -> Ready"));
}

#[test]
fn test_can_accept_requests() {
    let ready_state = MachineState::Ready;
    assert!(ready_state.can_accept_requests());

    let validating_state = MachineState::Validating {
        document: url("test.tx"),
    };
    assert!(!validating_state.can_accept_requests());

    let indexing_state = MachineState::Indexing;
    assert!(!indexing_state.can_accept_requests());
}

#[test]
fn test_is_validating() {
    let ready_state = MachineState::Ready;
    assert!(!ready_state.is_validating());

    let validating_state = MachineState::Validating {
        document: url("test.tx"),
    };
    assert!(validating_state.is_validating());

    let revalidating_state = MachineState::Revalidating {
        documents: vec![url("test.tx")],
        current: 0,
    };
    assert!(revalidating_state.is_validating());
}

#[test]
fn test_state_description() {
    assert_eq!(MachineState::Ready.description(), "Ready");
    assert_eq!(MachineState::Indexing.description(), "Indexing workspace");

    let uri = url("test.tx");
    let validating = MachineState::Validating {
        document: uri.clone(),
    };
    let desc = validating.description();
    assert!(desc.contains("Validating"));
    assert!(desc.contains("test.tx"));
}

#[test]
fn test_events_ignored_when_not_ready() {
    let mut workspace = WorkspaceState::new();

    // Try to open document before initialization completes
    workspace.process_event(StateEvent::Initialize);
    let uri = url("test.tx");
    let actions = workspace.process_event(StateEvent::DocumentOpened {
        uri: uri.clone(),
        content: "".to_string(),
    });

    // Should not transition from Indexing because it can't accept requests
    assert_eq!(*workspace.get_machine_state(), MachineState::Indexing);
    assert!(actions.is_empty());
}

#[test]
fn test_validation_completed_unexpected_state() {
    let mut workspace = WorkspaceState::new();
    workspace.process_event(StateEvent::Initialize);
    workspace.process_event(StateEvent::IndexingComplete);

    // Send validation completed without being in validating state
    let uri = url("test.tx");
    let diagnostics = vec![diagnostic("error", DiagnosticSeverity::ERROR)];
    let actions = workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri.clone(),
        diagnostics: diagnostics.clone(),
        success: false,
    });

    // Should still publish diagnostics even in unexpected state
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        StateAction::PublishDiagnostics { .. }
    ));

    // State should remain Ready
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);
}

#[test]
fn test_state_machine_with_shared_workspace() {
    let workspace = SharedWorkspaceState::new();

    // Initialize
    {
        let mut w = workspace.write();
        w.process_event(StateEvent::Initialize);
    }

    // Check state
    {
        let r = workspace.read();
        assert_eq!(*r.get_machine_state(), MachineState::Indexing);
    }

    // Complete indexing
    {
        let mut w = workspace.write();
        w.process_event(StateEvent::IndexingComplete);
    }

    // Verify Ready
    {
        let r = workspace.read();
        assert_eq!(*r.get_machine_state(), MachineState::Ready);
    }
}

#[test]
fn test_full_workflow() {
    let mut workspace = WorkspaceState::new();

    // 1. Initialize
    workspace.process_event(StateEvent::Initialize);
    assert_eq!(*workspace.get_machine_state(), MachineState::Indexing);

    // 2. Indexing completes
    workspace.process_event(StateEvent::IndexingComplete);
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);

    // 3. Open a document
    let uri = url("deploy.tx");
    workspace.process_event(StateEvent::DocumentOpened {
        uri: uri.clone(),
        content: "action \"deploy\" {}".to_string(),
    });
    assert!(matches!(
        *workspace.get_machine_state(),
        MachineState::Validating { .. }
    ));

    // 4. Validation completes
    workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri.clone(),
        diagnostics: vec![],
        success: true,
    });
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);

    // 5. Change document
    workspace.process_event(StateEvent::DocumentChanged {
        uri: uri.clone(),
        content: "action \"deploy\" { description = \"updated\" }".to_string(),
    });
    assert!(matches!(
        *workspace.get_machine_state(),
        MachineState::Validating { .. }
    ));

    // 6. Validation completes again
    workspace.process_event(StateEvent::ValidationCompleted {
        uri: uri.clone(),
        diagnostics: vec![],
        success: true,
    });
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);

    // 7. Close document
    workspace.process_event(StateEvent::DocumentClosed { uri });
    assert_eq!(*workspace.get_machine_state(), MachineState::Ready);

    // Verify state history has all transitions
    let history = workspace.get_state_history();
    assert!(history.transitions().len() >= 6);
}

#[test]
fn test_state_history_bounded() {
    use crate::cli::lsp::workspace::StateHistory;

    let mut history = StateHistory::new(3);

    // Add 5 transitions
    for i in 0..5 {
        let transition = crate::cli::lsp::workspace::StateTransition::new(
            MachineState::Ready,
            MachineState::Indexing,
            format!("Event {}", i),
        );
        history.record(transition);
    }

    // Should only keep last 3
    assert_eq!(history.transitions().len(), 3);
    assert_eq!(history.transitions()[0].event, "Event 2");
    assert_eq!(history.transitions()[2].event, "Event 4");
}
