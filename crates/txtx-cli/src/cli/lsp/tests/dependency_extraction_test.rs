//! TDD tests for automatic dependency extraction from HCL content.

use super::mock_editor::MockEditor;
use super::test_utils::url;
use lsp_types::Url;

#[test]
fn test_extract_manifest_dependency() {
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

    // Open runbook that references manifest inputs
    editor.open_document(
        runbook_uri.clone(),
        r#"
variable "key" {
    value = input.api_key
}
"#
        .to_string(),
    );

    // Should automatically detect runbook depends on manifest
    editor.assert_dependency(&runbook_uri, &manifest_uri);
}

#[test]
fn test_extract_output_dependency() {
    let mut editor = MockEditor::new();
    let action_a = url("action_a.tx");
    let action_b = url("action_b.tx");

    editor.open_document(
        action_a.clone(),
        r#"
action "deploy" "evm::call" {
    contract_address = "0x123"
}
"#
        .to_string(),
    );

    // action_b depends on action_a via output reference
    editor.open_document(
        action_b.clone(),
        r#"
action "verify" "evm::call" {
    contract_address = output.deploy.address
}
"#
        .to_string(),
    );

    // Should detect action_b depends on action_a
    editor.assert_dependency(&action_b, &action_a);
}

#[test]
fn test_extract_variable_dependency() {
    let mut editor = MockEditor::new();
    let file_a = url("a.tx");
    let file_b = url("b.tx");

    editor.open_document(
        file_a.clone(),
        r#"
variable "base_url" {
    value = "https://api.example.com"
}
"#
        .to_string(),
    );

    editor.open_document(
        file_b.clone(),
        r#"
variable "full_url" {
    value = "${variable.base_url}/v1/endpoint"
}
"#
        .to_string(),
    );

    // Should detect file_b depends on file_a
    editor.assert_dependency(&file_b, &file_a);
}

#[test]
fn test_no_dependency_when_self_contained() {
    let mut editor = MockEditor::new();
    let runbook_uri = url("standalone.tx");

    editor.open_document(
        runbook_uri.clone(),
        r#"
action "deploy" "evm::call" {
    contract_address = "0x123"
}

variable "local" {
    value = "local_value"
}
"#
        .to_string(),
    );

    // Should have no dependencies
    {
        let workspace = editor.workspace().read();
        let deps = workspace.dependencies().get_dependencies(&runbook_uri);
        assert!(
            deps.is_none() || deps.unwrap().is_empty(),
            "Self-contained runbook should have no dependencies"
        );
    }
}

#[test]
fn test_extract_multiple_dependencies() {
    let mut editor = MockEditor::new();
    let manifest_uri = url("txtx.yml");
    let base_uri = url("base.tx");
    let derived_uri = url("derived.tx");

    editor.open_document(
        manifest_uri.clone(),
        r#"
runbooks:
  - name: derived
    location: derived.tx
environments:
  production:
    api_key: "prod_key"
"#
        .to_string(),
    );

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
    value = "${input.api_key}_${variable.base}"
}
"#
        .to_string(),
    );

    // Should detect derived depends on both manifest and base
    editor.assert_dependency(&derived_uri, &manifest_uri);
    editor.assert_dependency(&derived_uri, &base_uri);
}

#[test]
fn test_dependency_extraction_on_document_change() {
    let mut editor = MockEditor::new();
    let file_a = url("a.tx");
    let file_b = url("b.tx");

    // Initially, file_b has no dependencies
    editor.open_document(
        file_b.clone(),
        r#"
variable "standalone" {
    value = "standalone_value"
}
"#
        .to_string(),
    );

    {
        let workspace = editor.workspace().read();
        let deps = workspace.dependencies().get_dependencies(&file_b);
        assert!(
            deps.is_none() || deps.unwrap().is_empty(),
            "Should have no dependencies initially"
        );
    }

    // Open file_a
    editor.open_document(
        file_a.clone(),
        r#"
variable "base" {
    value = "base_value"
}
"#
        .to_string(),
    );

    // Now change file_b to depend on file_a
    editor.change_document(
        &file_b,
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    // Should now detect dependency
    editor.assert_dependency(&file_b, &file_a);
}

#[test]
fn test_dependency_removed_on_content_change() {
    let mut editor = MockEditor::new();
    let file_a = url("a.tx");
    let file_b = url("b.tx");

    editor.open_document(
        file_a.clone(),
        r#"
variable "base" {
    value = "base_value"
}
"#
        .to_string(),
    );

    // file_b initially depends on file_a
    editor.open_document(
        file_b.clone(),
        r#"
variable "derived" {
    value = variable.base
}
"#
        .to_string(),
    );

    editor.assert_dependency(&file_b, &file_a);

    // Change file_b to not depend on file_a anymore
    editor.change_document(
        &file_b,
        r#"
variable "standalone" {
    value = "standalone_value"
}
"#
        .to_string(),
    );

    // Dependency should be removed
    {
        let workspace = editor.workspace().read();
        let deps = workspace.dependencies().get_dependencies(&file_b);
        assert!(
            deps.is_none() || !deps.unwrap().contains(&file_a),
            "Dependency should be removed after content change"
        );
    }
}
