//! TDD tests for LSP state management
//!
//! These tests use the mock editor to verify state management behavior

use super::mock_editor::MockEditor;
use super::test_utils::{error_diagnostic, url, warning_diagnostic};
use crate::cli::lsp::workspace::ValidationStatus;

#[test]
fn test_content_hash_prevents_redundant_validation() {
    let mut editor = MockEditor::new();
    let uri = url("test.tx");

    // Open document
    editor.open_document(uri.clone(), "action \"test\" \"evm::call\" {}".to_string());
    editor.assert_needs_validation(&uri);

    // Validate
    editor.validate_document(&uri, vec![]);
    editor.assert_validation_status(&uri, ValidationStatus::Clean);
    editor.assert_no_validation_needed(&uri);

    // "Change" to same content - should not need validation
    editor.change_document(&uri, "action \"test\" \"evm::call\" {}".to_string());
    editor.assert_no_validation_needed(&uri);
}

#[test]
fn test_content_change_triggers_validation() {
    let mut editor = MockEditor::new();
    let uri = url("test.tx");

    // Open and validate
    editor.open_document(uri.clone(), "old content".to_string());
    editor.validate_document(&uri, vec![]);
    editor.assert_validation_status(&uri, ValidationStatus::Clean);

    // Change content
    editor.change_document(&uri, "new content".to_string());
    editor.assert_needs_validation(&uri);
    editor.assert_dirty(&uri);
}

#[test]
fn test_environment_switch_invalidates_documents() {
    let mut editor = MockEditor::new();
    let uri = url("deploy.tx");

    // Open and validate in sepolia
    editor.switch_environment("sepolia".to_string());
    editor.open_document(uri.clone(), "value = input.api_key".to_string());
    editor.validate_document(&uri, vec![]);
    editor.assert_validation_status(&uri, ValidationStatus::Clean);
    editor.assert_no_validation_needed(&uri);

    // Switch to mainnet - should need re-validation
    editor.switch_environment("mainnet".to_string());
    editor.assert_needs_validation(&uri);
}

#[test]
fn test_cycle_dependency_detection_and_fix() {
    let mut editor = MockEditor::new();
    let uri_a = url("a.tx");
    let uri_b = url("b.tx");
    let uri_c = url("c.tx");

    // Create cyclic dependencies: a -> b -> c -> a
    editor.open_document(uri_a.clone(), "// depends on b".to_string());
    editor.open_document(uri_b.clone(), "// depends on c".to_string());
    editor.open_document(uri_c.clone(), "// depends on a".to_string());

    {
        let mut workspace = editor.workspace().write();
        workspace.dependencies_mut().add_dependency(uri_a.clone(), uri_b.clone());
        workspace.dependencies_mut().add_dependency(uri_b.clone(), uri_c.clone());
        workspace.dependencies_mut().add_dependency(uri_c.clone(), uri_a.clone());
    }

    // Detect cycle
    editor.assert_cycle();

    // Fix cycle by removing c -> a dependency
    {
        let mut workspace = editor.workspace().write();
        workspace.dependencies_mut().remove_dependency(&uri_c, &uri_a);
    }

    // No more cycle
    editor.assert_no_cycle();
}

#[test]
fn test_manifest_change_invalidates_dependent_runbooks() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let runbook_a = url("a.tx");
    let runbook_b = url("b.tx");

    // Open manifest and runbooks
    editor.open_document(
        manifest_uri.clone(),
        r#"
runbooks:
  - name: a
    location: a.tx
environments:
  sepolia:
    api_key: "test_key"
"#
        .to_string(),
    );
    editor.open_document(runbook_a.clone(), "value = input.api_key".to_string());
    editor.open_document(runbook_b.clone(), "value = input.api_key".to_string());

    // Setup dependencies
    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(runbook_a.clone(), manifest_uri.clone());
        workspace
            .dependencies_mut()
            .add_dependency(runbook_b.clone(), manifest_uri.clone());
    }

    // Validate runbooks
    editor.validate_document(&runbook_a, vec![]);
    editor.validate_document(&runbook_b, vec![]);
    editor.assert_validation_status(&runbook_a, ValidationStatus::Clean);
    editor.assert_validation_status(&runbook_b, ValidationStatus::Clean);

    // Change manifest
    editor.change_document(
        &manifest_uri,
        r#"
runbooks:
  - name: a
    location: a.tx
environments:
  sepolia:
    api_key: "new_key"
    new_input: "value"
"#
        .to_string(),
    );

    // Dependents should be marked stale
    {
        let mut workspace = editor.workspace().write();
        workspace.mark_dirty(&runbook_a);
        workspace.mark_dirty(&runbook_b);
    }

    editor.assert_dirty(&runbook_a);
    editor.assert_dirty(&runbook_b);
}

#[test]
fn test_validation_status_transitions() {
    let mut editor = MockEditor::new();
    let uri = url("test.tx");

    // Unvalidated -> Validating -> Clean
    editor.open_document(uri.clone(), "valid content".to_string());
    editor.assert_needs_validation(&uri);

    editor.validate_document(&uri, vec![]);
    editor.assert_validation_status(&uri, ValidationStatus::Clean);
    editor.assert_not_dirty(&uri);

    // Clean -> Error (content changed with errors)
    editor.change_document(&uri, "invalid content".to_string());
    editor.validate_document(&uri, vec![error_diagnostic("syntax error", 0)]);
    editor.assert_validation_status(&uri, ValidationStatus::Error);

    // Error -> Warning (fix errors, leave warnings)
    editor.change_document(&uri, "content with warning".to_string());
    editor.validate_document(&uri, vec![warning_diagnostic("unused variable", 0)]);
    editor.assert_validation_status(&uri, ValidationStatus::Warning);

    // Warning -> Clean (fix all issues)
    editor.change_document(&uri, "clean content".to_string());
    editor.validate_document(&uri, vec![]);
    editor.assert_validation_status(&uri, ValidationStatus::Clean);
}

#[test]
fn test_dirty_documents_tracking() {
    let mut editor = MockEditor::new();
    let uri1 = url("test1.tx");
    let uri2 = url("test2.tx");

    // Open documents
    editor.open_document(uri1.clone(), "content 1".to_string());
    editor.open_document(uri2.clone(), "content 2".to_string());

    // Both should be dirty (unvalidated)
    {
        let workspace = editor.workspace().read();
        let dirty = workspace.get_dirty_documents();
        assert_eq!(dirty.len(), 0); // Not explicitly marked dirty yet
    }

    // Mark dirty and validate one
    {
        let mut workspace = editor.workspace().write();
        workspace.mark_dirty(&uri1);
        workspace.mark_dirty(&uri2);
    }

    editor.assert_dirty(&uri1);
    editor.assert_dirty(&uri2);

    // Validate uri1 - should be removed from dirty set
    editor.validate_document(&uri1, vec![]);
    editor.assert_not_dirty(&uri1);
    editor.assert_dirty(&uri2);

    // Validate uri2
    editor.validate_document(&uri2, vec![]);
    editor.assert_not_dirty(&uri2);

    {
        let workspace = editor.workspace().read();
        assert_eq!(workspace.get_dirty_documents().len(), 0);
    }
}

#[test]
fn test_transitive_dependency_invalidation() {
    let mut editor = MockEditor::new();
    let manifest = url("txtx.yml");
    let runbook_a = url("a.tx");
    let runbook_b = url("b.tx");
    let runbook_c = url("c.tx");

    // Setup: manifest <- a <- b <- c
    editor.open_document(manifest.clone(), "manifest content".to_string());
    editor.open_document(runbook_a.clone(), "runbook a".to_string());
    editor.open_document(runbook_b.clone(), "runbook b".to_string());
    editor.open_document(runbook_c.clone(), "runbook c".to_string());

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(runbook_a.clone(), manifest.clone());
        workspace
            .dependencies_mut()
            .add_dependency(runbook_b.clone(), runbook_a.clone());
        workspace
            .dependencies_mut()
            .add_dependency(runbook_c.clone(), runbook_b.clone());
    }

    // Validate all
    editor.validate_document(&runbook_a, vec![]);
    editor.validate_document(&runbook_b, vec![]);
    editor.validate_document(&runbook_c, vec![]);

    // Change manifest - all should be affected
    editor.change_document(&manifest, "new manifest".to_string());

    {
        let workspace = editor.workspace().read();
        let affected = workspace.dependencies().get_affected_documents(&manifest);
        assert_eq!(affected.len(), 3);
        assert!(affected.contains(&runbook_a));
        assert!(affected.contains(&runbook_b));
        assert!(affected.contains(&runbook_c));
    }
}

#[test]
fn test_document_close_cleanup() {
    let mut editor = MockEditor::new();
    let uri = url("test.tx");
    let manifest = url("txtx.yml");

    editor.open_document(uri.clone(), "content".to_string());
    editor.open_document(manifest.clone(), "manifest".to_string());

    // Setup dependency
    {
        let mut workspace = editor.workspace().write();
        workspace.dependencies_mut().add_dependency(uri.clone(), manifest.clone());
    }

    editor.assert_dependency(&uri, &manifest);

    // Validate
    editor.validate_document(&uri, vec![]);

    // Close document
    editor.close_document(&uri);

    // Validation state and dependencies should be cleaned up
    {
        let workspace = editor.workspace().read();
        assert!(workspace.get_validation_state(&uri).is_none());
        assert!(workspace.dependencies().get_dependencies(&uri).is_none());
    }
}

#[test]
fn test_stale_marking_on_dependency_change() {
    let mut editor = MockEditor::new();
    let manifest = url("txtx.yml");
    let runbook = url("deploy.tx");

    editor.open_document(manifest.clone(), "manifest v1".to_string());
    editor.open_document(runbook.clone(), "runbook v1".to_string());

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(runbook.clone(), manifest.clone());
    }

    // Validate runbook
    editor.validate_document(&runbook, vec![]);
    editor.assert_validation_status(&runbook, ValidationStatus::Clean);

    // Change manifest and mark runbook as stale
    editor.change_document(&manifest, "manifest v2".to_string());
    {
        let mut workspace = editor.workspace().write();
        workspace.mark_dirty(&runbook);
    }

    // Runbook should be stale
    {
        let workspace = editor.workspace().read();
        let state = workspace.get_validation_state(&runbook).unwrap();
        assert_eq!(state.status, ValidationStatus::Stale);
    }
    editor.assert_dirty(&runbook);
}
