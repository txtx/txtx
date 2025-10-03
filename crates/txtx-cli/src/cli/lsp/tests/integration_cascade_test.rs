//! Integration tests for Phase 4: cascade validation through LSP handlers
//!
//! These tests verify that the dependency tracking and cascade validation
//! implemented in Phases 1-3 are properly integrated with the LSP handlers.

use super::mock_editor::MockEditor;
use super::test_utils::url;

#[test]
fn test_manifest_change_triggers_dependent_validation() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let runbook_uri = url("deploy.tx");

    // Open manifest
    editor.open_document(
        manifest_uri.clone(),
        r#"
runbooks:
  - name: deploy
    location: deploy.tx
environments:
  production:
    api_key: "prod_key"
"#
        .to_string(),
    );

    // Open runbook that uses manifest inputs
    editor.open_document(
        runbook_uri.clone(),
        r#"
variable "key" {
    value = input.api_key
}
"#
        .to_string(),
    );

    // Runbook should be marked as clean after initial validation
    editor.clear_dirty();

    // Change manifest
    editor.change_document(
        &manifest_uri,
        r#"
runbooks:
  - name: deploy
    location: deploy.tx
environments:
  production:
    api_key: "new_prod_key"
"#
        .to_string(),
    );

    // Runbook should now be marked dirty (needs re-validation)
    editor.assert_is_dirty(&runbook_uri);
}

#[test]
fn test_action_definition_change_cascades() {
    let mut editor = MockEditor::new();
    let action_def = url("deploy.tx");
    let action_user = url("verify.tx");

    // Open file that defines an action
    editor.open_document(
        action_def.clone(),
        r#"
action "deploy" "evm::call" {
    contract_address = "0x123"
}
"#
        .to_string(),
    );

    // Open file that uses that action's output
    editor.open_document(
        action_user.clone(),
        r#"
action "verify" "evm::call" {
    contract_address = output.deploy.address
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Change the action definition
    editor.change_document(
        &action_def,
        r#"
action "deploy" "evm::call" {
    contract_address = "0x456"
}
"#
        .to_string(),
    );

    // User file should be marked dirty
    editor.assert_is_dirty(&action_user);
}

#[test]
fn test_variable_definition_change_cascades() {
    let mut editor = MockEditor::new();
    let var_def = url("base.tx");
    let var_user = url("derived.tx");

    editor.open_document(
        var_def.clone(),
        r#"
variable "base_url" {
    value = "https://api.example.com"
}
"#
        .to_string(),
    );

    editor.open_document(
        var_user.clone(),
        r#"
variable "full_url" {
    value = "${variable.base_url}/v1/endpoint"
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Change the variable definition
    editor.change_document(
        &var_def,
        r#"
variable "base_url" {
    value = "https://api.newdomain.com"
}
"#
        .to_string(),
    );

    // User file should be marked dirty
    editor.assert_is_dirty(&var_user);
}

#[test]
fn test_transitive_cascade_through_handlers() {
    let mut editor = MockEditor::new();
    let bottom = url("bottom.tx");
    let middle = url("middle.tx");
    let top = url("top.tx");

    // bottom.tx defines a variable
    editor.open_document(
        bottom.clone(),
        r#"
variable "base" {
    value = "base_value"
}
"#
        .to_string(),
    );

    // middle.tx uses bottom's variable and defines its own
    editor.open_document(
        middle.clone(),
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    // top.tx uses middle's variable
    editor.open_document(
        top.clone(),
        r#"
variable "final" {
    value = variable.derived
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Change bottom.tx
    editor.change_document(
        &bottom,
        r#"
variable "base" {
    value = "new_base_value"
}
"#
        .to_string(),
    );

    // Both middle and top should be marked dirty (transitive cascade)
    editor.assert_is_dirty(&middle);
    editor.assert_is_dirty(&top);
}

#[test]
fn test_environment_change_marks_all_runbooks_dirty() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let runbook1 = url("deploy.tx");
    let runbook2 = url("config.tx");

    // Open manifest with multiple environments
    editor.open_document(
        manifest_uri.clone(),
        r#"
runbooks:
  - name: deploy
    location: deploy.tx
  - name: config
    location: config.tx
environments:
  dev:
    api_key: "dev_key"
  prod:
    api_key: "prod_key"
"#
        .to_string(),
    );

    // Open runbooks that use environment inputs
    editor.open_document(
        runbook1.clone(),
        r#"
variable "key" {
    value = input.api_key
}
"#
        .to_string(),
    );

    editor.open_document(
        runbook2.clone(),
        r#"
variable "api" {
    value = input.api_key
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Set environment to "dev"
    editor.set_environment(Some("dev".to_string()));

    // All runbooks should be marked dirty
    editor.assert_is_dirty(&runbook1);
    editor.assert_is_dirty(&runbook2);
}

#[test]
fn test_cascade_validation_publishes_diagnostics() {
    let mut editor = MockEditor::new();
    let base = url("base.tx");
    let derived = url("derived.tx");

    editor.open_document(
        base.clone(),
        r#"
variable "base" {
    value = "base_value"
}
"#
        .to_string(),
    );

    editor.open_document(
        derived.clone(),
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Change base to trigger cascade
    editor.change_document(
        &base,
        r#"
variable "base" {
    value = "new_value"
}
"#
        .to_string(),
    );

    // Derived should be dirty
    editor.assert_is_dirty(&derived);

    // After validation, dirty should be cleared
    // (This will be tested when we integrate with actual validation)
}

#[test]
fn test_no_cascade_for_independent_files() {
    let mut editor = MockEditor::new();
    let file1 = url("standalone1.tx");
    let file2 = url("standalone2.tx");

    editor.open_document(
        file1.clone(),
        r#"
variable "var1" {
    value = "value1"
}
"#
        .to_string(),
    );

    editor.open_document(
        file2.clone(),
        r#"
variable "var2" {
    value = "value2"
}
"#
        .to_string(),
    );

    editor.clear_dirty();

    // Change file1
    editor.change_document(
        &file1,
        r#"
variable "var1" {
    value = "new_value1"
}
"#
        .to_string(),
    );

    // Only file1 should be dirty, not file2
    editor.assert_is_dirty(&file1);
    {
        let workspace = editor.workspace().read();
        assert!(
            !workspace.get_dirty_documents().contains(&file2),
            "Independent file should not be marked dirty"
        );
    }
}

#[test]
fn test_dependency_extraction_on_open() {
    let mut editor = MockEditor::new();
    let action_def = url("actions.tx");
    let action_user = url("user.tx");

    // Open action definition first
    editor.open_document(
        action_def.clone(),
        r#"
action "deploy" "evm::call" {
    contract_address = "0x123"
}
"#
        .to_string(),
    );

    // Open file that uses the action - dependency should be auto-extracted
    editor.open_document(
        action_user.clone(),
        r#"
action "verify" "evm::call" {
    result = output.deploy.result
}
"#
        .to_string(),
    );

    // Verify dependency was extracted
    editor.assert_dependency(&action_user, &action_def);
}

#[test]
fn test_dependency_update_on_change() {
    let mut editor = MockEditor::new();
    let file_a = url("a.tx");
    let file_b = url("b.tx");
    let file_c = url("c.tx");

    // file_a defines a variable
    editor.open_document(
        file_a.clone(),
        r#"
variable "var_a" {
    value = "a"
}
"#
        .to_string(),
    );

    // file_c defines a variable
    editor.open_document(
        file_c.clone(),
        r#"
variable "var_c" {
    value = "c"
}
"#
        .to_string(),
    );

    // file_b initially depends on file_a
    editor.open_document(
        file_b.clone(),
        r#"
variable "var_b" {
    value = variable.var_a
}
"#
        .to_string(),
    );

    editor.assert_dependency(&file_b, &file_a);

    // Change file_b to depend on file_c instead
    editor.change_document(
        &file_b,
        r#"
variable "var_b" {
    value = variable.var_c
}
"#
        .to_string(),
    );

    // Should now depend on file_c, not file_a
    editor.assert_dependency(&file_b, &file_c);
    {
        let workspace = editor.workspace().read();
        let deps = workspace.dependencies().get_dependencies(&file_b);
        assert!(
            deps.is_none() || !deps.unwrap().contains(&file_a),
            "Old dependency should be removed"
        );
    }
}
