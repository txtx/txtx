//! TDD tests for cascade validation when dependencies change.

use super::mock_editor::MockEditor;
use super::test_utils::{error_diagnostic, url};
use crate::cli::lsp::workspace::ValidationStatus;

#[test]
fn test_cascade_validation_on_manifest_change() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let runbook_uri = url("deploy.tx");

    // Setup: manifest and runbook with dependency
    editor.open_document(
        manifest_uri.clone(),
        r#"
environments:
  production:
    api_key: "prod_key"
"#
        .to_string(),
    );

    editor.open_document(
        runbook_uri.clone(),
        r#"
variable "key" {
    value = input.api_key
}
"#
        .to_string(),
    );

    // Manually establish dependency (will be automatic later)
    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(runbook_uri.clone(), manifest_uri.clone());
    }

    // Validate both documents
    editor.validate_document(&manifest_uri, vec![]);
    editor.validate_document(&runbook_uri, vec![]);
    editor.assert_validation_status(&runbook_uri, ValidationStatus::Clean);

    // Change manifest
    editor.change_document(
        &manifest_uri,
        r#"
environments:
  production:
    api_key: "new_prod_key"
    new_input: "value"
"#
        .to_string(),
    );

    // Runbook should be marked dirty
    editor.assert_dirty(&runbook_uri);
}

#[test]
fn test_cascade_validation_with_errors() {
    let mut editor = MockEditor::new();
    let base_uri = url("base.tx");
    let derived_uri = url("derived.tx");

    editor.open_document(
        base_uri.clone(),
        r#"
variable "base" {
    value = "base_value"
}
"#
        .to_string(),
    );

    editor.open_document(
        derived_uri.clone(),
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(derived_uri.clone(), base_uri.clone());
    }

    // Validate both
    editor.validate_document(&base_uri, vec![]);
    editor.validate_document(&derived_uri, vec![]);

    // Change base to have errors
    editor.change_document(
        &base_uri,
        r#"
variable "base" {
    invalid syntax here
}
"#
        .to_string(),
    );

    // Simulate validation with error
    editor.validate_document(&base_uri, vec![error_diagnostic("syntax error", 2)]);

    // Derived should be marked dirty even though its content didn't change
    editor.assert_dirty(&derived_uri);
}

#[test]
fn test_transitive_cascade_validation() {
    let mut editor = MockEditor::new();
    let base_uri = url("base.tx");
    let middle_uri = url("middle.tx");
    let top_uri = url("top.tx");

    // Chain: base <- middle <- top
    editor.open_document(
        base_uri.clone(),
        r#"
variable "base" {
    value = "base"
}
"#
        .to_string(),
    );

    editor.open_document(
        middle_uri.clone(),
        r#"
variable "middle" {
    value = variable.base
}
"#
        .to_string(),
    );

    editor.open_document(
        top_uri.clone(),
        r#"
variable "top" {
    value = variable.middle
}
"#
        .to_string(),
    );

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(middle_uri.clone(), base_uri.clone());
        workspace
            .dependencies_mut()
            .add_dependency(top_uri.clone(), middle_uri.clone());
    }

    // Validate all
    editor.validate_document(&base_uri, vec![]);
    editor.validate_document(&middle_uri, vec![]);
    editor.validate_document(&top_uri, vec![]);

    // Change base
    editor.change_document(
        &base_uri,
        r#"
variable "base" {
    value = "new_base"
}
"#
        .to_string(),
    );

    // Both middle and top should be marked dirty (transitive)
    editor.assert_dirty(&middle_uri);
    editor.assert_dirty(&top_uri);
}

#[test]
fn test_no_cascade_on_independent_change() {
    let mut editor = MockEditor::new();
    let file_a = url("a.tx");
    let file_b = url("b.tx");

    // Two independent files
    editor.open_document(
        file_a.clone(),
        r#"
variable "a" {
    value = "a_value"
}
"#
        .to_string(),
    );

    editor.open_document(
        file_b.clone(),
        r#"
variable "b" {
    value = "b_value"
}
"#
        .to_string(),
    );

    // Validate both
    editor.validate_document(&file_a, vec![]);
    editor.validate_document(&file_b, vec![]);

    // Change file_a
    editor.change_document(
        &file_a,
        r#"
variable "a" {
    value = "new_a_value"
}
"#
        .to_string(),
    );

    // file_b should NOT be marked dirty (no dependency)
    editor.assert_not_dirty(&file_b);
}

#[test]
fn test_cascade_validation_multiple_dependents() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let runbook_a = url("a.tx");
    let runbook_b = url("b.tx");
    let runbook_c = url("c.tx");

    editor.open_document(
        manifest_uri.clone(),
        r#"
environments:
  production:
    api_key: "key"
"#
        .to_string(),
    );

    editor.open_document(runbook_a.clone(), "value = input.api_key".to_string());
    editor.open_document(runbook_b.clone(), "value = input.api_key".to_string());
    editor.open_document(runbook_c.clone(), "value = input.api_key".to_string());

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(runbook_a.clone(), manifest_uri.clone());
        workspace
            .dependencies_mut()
            .add_dependency(runbook_b.clone(), manifest_uri.clone());
        workspace
            .dependencies_mut()
            .add_dependency(runbook_c.clone(), manifest_uri.clone());
    }

    // Validate all
    editor.validate_document(&manifest_uri, vec![]);
    editor.validate_document(&runbook_a, vec![]);
    editor.validate_document(&runbook_b, vec![]);
    editor.validate_document(&runbook_c, vec![]);

    // Change manifest
    editor.change_document(
        &manifest_uri,
        r#"
environments:
  production:
    api_key: "new_key"
"#
        .to_string(),
    );

    // All three runbooks should be marked dirty
    editor.assert_dirty(&runbook_a);
    editor.assert_dirty(&runbook_b);
    editor.assert_dirty(&runbook_c);
}

#[test]
fn test_cascade_validation_clears_after_revalidation() {
    let mut editor = MockEditor::new();
    let base_uri = url("base.tx");
    let derived_uri = url("derived.tx");

    editor.open_document(
        base_uri.clone(),
        r#"
variable "base" {
    value = "base"
}
"#
        .to_string(),
    );

    editor.open_document(
        derived_uri.clone(),
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    {
        let mut workspace = editor.workspace().write();
        workspace
            .dependencies_mut()
            .add_dependency(derived_uri.clone(), base_uri.clone());
    }

    // Validate both
    editor.validate_document(&base_uri, vec![]);
    editor.validate_document(&derived_uri, vec![]);

    // Change base
    editor.change_document(
        &base_uri,
        r#"
variable "base" {
    value = "new_base"
}
"#
        .to_string(),
    );

    editor.assert_dirty(&derived_uri);

    // Re-validate base
    editor.validate_document(&base_uri, vec![]);

    // derived is still dirty (needs its own validation)
    editor.assert_dirty(&derived_uri);

    // Re-validate derived
    editor.validate_document(&derived_uri, vec![]);

    // Now derived should not be dirty
    editor.assert_not_dirty(&derived_uri);
}
