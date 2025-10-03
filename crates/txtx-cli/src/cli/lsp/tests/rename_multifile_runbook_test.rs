//! Tests for renaming inputs across multifile runbooks.
//!
//! These tests verify that when renaming an input reference, the rename operation
//! correctly discovers and updates:
//! - All files within a multifile runbook directory (both open and closed files)
//! - All multifile runbooks defined in the manifest
//! - Files in nested subdirectory structures
//!
//! A multifile runbook is a directory containing multiple `.tx` files that together
//! define a complete runbook, as specified in the manifest's `location` field.

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::RenameHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{
        Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url,
        WorkDoneProgressParams, WorkspaceEdit,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// Helper to create a workspace with manifest and handler.
    fn setup_workspace(
        manifest_content: &str,
        workspace_root: &Path,
    ) -> (SharedWorkspaceState, RenameHandler, Url) {
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state
            .write()
            .open_document(manifest_uri.clone(), manifest_content.to_string());

        (workspace_state, handler, manifest_uri)
    }

    /// Helper to create a runbook directory with files.
    fn create_runbook_files(runbook_dir: &Path, files: &[(&str, &str)]) {
        fs::create_dir_all(runbook_dir).unwrap();
        for (filename, content) in files {
            fs::write(runbook_dir.join(filename), content).unwrap();
        }
    }

    /// Helper to create rename parameters for a position in a document.
    fn create_rename_params(uri: Url, line: u32, character: u32, new_name: &str) -> RenameParams {
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Helper to assert a URI has exactly the expected number of edits with the expected new text.
    fn assert_edits(
        changes: &std::collections::HashMap<Url, Vec<lsp_types::TextEdit>>,
        uri: &Url,
        expected_count: usize,
        expected_text: &str,
        message: &str,
    ) {
        assert!(changes.contains_key(uri), "{}", message);
        let edits = &changes[uri];
        assert_eq!(
            edits.len(),
            expected_count,
            "{}: expected {} edits, got {}",
            message,
            expected_count,
            edits.len()
        );
        for edit in edits {
            assert_eq!(
                edit.new_text, expected_text,
                "{}: expected '{}', got '{}'",
                message, expected_text, edit.new_text
            );
        }
    }

    /// Tests renaming an input from within a multifile runbook file.
    ///
    /// This test verifies that when clicking on an input reference in an open `.tx` file,
    /// the rename operation updates:
    /// - The manifest (all environments)
    /// - The open file where the rename was initiated
    /// - All closed files in the same multifile runbook directory
    #[test]
    fn test_rename_input_across_multifile_runbook() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let manifest_content = r#"
runbooks:
  - name: deploy
    location: ./runbook

environments:
  global:
    network_id: 1
    api_url: "https://api.example.com"
  sepolia:
    network_id: 11155111
    api_url: "https://api.sepolia.example.com"
"#;

        let (workspace_state, handler, manifest_uri) =
            setup_workspace(manifest_content, workspace_root);

        // Create multifile runbook with main.tx, config.tx, and outputs.tx
        let runbook_dir = workspace_root.join("runbook");
        let main_content = r#"
addon "evm" {
    network_id = input.network_id
    rpc_url = input.api_url
}

action "deploy" "evm::deploy_contract" {
    bytecode = "0x1234"
}
"#;

        create_runbook_files(
            &runbook_dir,
            &[
                ("main.tx", main_content),
                (
                    "config.tx",
                    r#"
variable "explorer_url" {
    value = "https://explorer.example.com?network=${input.network_id}"
}
"#,
                ),
                (
                    "outputs.tx",
                    r#"
output "deployment_info" {
    value = "Deployed to network ${input.network_id} using ${input.api_url}"
}

output "explorer" {
    value = variable.explorer_url
}
"#,
                ),
            ],
        );

        // Open only main.tx (other runbook files remain closed)
        let main_uri = Url::from_file_path(runbook_dir.join("main.tx")).unwrap();
        workspace_state
            .write()
            .open_document(main_uri.clone(), main_content.to_string());

        // Rename "network_id" to "chain_id" from main.tx
        let params = create_rename_params(main_uri.clone(), 2, 23, "chain_id");

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Verify edits across all files
        let config_uri = Url::from_file_path(runbook_dir.join("config.tx")).unwrap();
        let outputs_uri = Url::from_file_path(runbook_dir.join("outputs.tx")).unwrap();

        assert_edits(&changes, &manifest_uri, 2, "chain_id", "manifest (global + sepolia)");
        assert_edits(&changes, &main_uri, 1, "chain_id", "main.tx (open file)");
        assert_edits(&changes, &config_uri, 1, "chain_id", "config.tx (closed file)");
        assert_edits(&changes, &outputs_uri, 1, "chain_id", "outputs.tx (closed file)");
    }

    /// Tests renaming an input from the manifest YAML key.
    ///
    /// This test verifies that when clicking on an input key in the manifest,
    /// the rename operation updates all files in all multifile runbooks,
    /// even when those files are closed.
    #[test]
    fn test_rename_input_from_manifest_affects_all_multifile_runbook_files() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let manifest_content = r#"
runbooks:
  - name: setup
    location: ./setup

environments:
  global:
    timeout: 30
  production:
    timeout: 60
"#;

        let (workspace_state, handler, manifest_uri) =
            setup_workspace(manifest_content, workspace_root);

        // Create multifile runbook with 3 files (all closed)
        let runbook_dir = workspace_root.join("setup");
        create_runbook_files(
            &runbook_dir,
            &[
                (
                    "file1.tx",
                    r#"
variable "max_wait" {
    value = input.timeout
}
"#,
                ),
                (
                    "file2.tx",
                    r#"
action "wait" "core::sleep" {
    duration = input.timeout
}
"#,
                ),
                (
                    "file3.tx",
                    r#"
output "config" {
    value = "Timeout set to ${input.timeout} seconds"
}
"#,
                ),
            ],
        );

        // Rename "timeout" to "max_duration" from manifest
        // Line 7 is "    timeout: 30" in global env (line 0 is blank from r#")
        let params = create_rename_params(manifest_uri.clone(), 7, 4, "max_duration");

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Verify all files were updated (even though all were closed)
        let file1_uri = Url::from_file_path(runbook_dir.join("file1.tx")).unwrap();
        let file2_uri = Url::from_file_path(runbook_dir.join("file2.tx")).unwrap();
        let file3_uri = Url::from_file_path(runbook_dir.join("file3.tx")).unwrap();

        assert_edits(&changes, &manifest_uri, 2, "max_duration", "manifest (global + production)");
        assert_edits(&changes, &file1_uri, 1, "max_duration", "file1.tx (closed)");
        assert_edits(&changes, &file2_uri, 1, "max_duration", "file2.tx (closed)");
        assert_edits(&changes, &file3_uri, 1, "max_duration", "file3.tx (closed)");
    }

    /// Tests renaming an input across multiple distinct multifile runbooks.
    ///
    /// This test verifies that when renaming an input from the manifest,
    /// the operation updates files in all multifile runbooks defined in the manifest,
    /// not just the first one.
    #[test]
    fn test_rename_input_with_multiple_multifile_runbooks() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let manifest_content = r#"
runbooks:
  - name: deploy
    location: ./deploy
  - name: test
    location: ./test

environments:
  global:
    api_key: "default_key"
"#;

        let (workspace_state, handler, manifest_uri) =
            setup_workspace(manifest_content, workspace_root);

        // Create first multifile runbook (deploy)
        let deploy_dir = workspace_root.join("deploy");
        create_runbook_files(
            &deploy_dir,
            &[(
                "main.tx",
                r#"
action "call_api" "http::post" {
    headers = { "Authorization": "Bearer ${input.api_key}" }
}
"#,
            )],
        );

        // Create second multifile runbook (test)
        let test_dir = workspace_root.join("test");
        create_runbook_files(
            &test_dir,
            &[
                (
                    "setup.tx",
                    r#"
variable "auth_header" {
    value = input.api_key
}
"#,
                ),
                (
                    "run.tx",
                    r#"
action "verify" "http::get" {
    url = "https://api.example.com/verify?key=${input.api_key}"
}
"#,
                ),
            ],
        );

        // Rename "api_key" to "auth_token" from manifest
        // Line 9 is "    api_key: "default_key"" (line 0 is blank from r#")
        let params = create_rename_params(manifest_uri.clone(), 9, 4, "auth_token");

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Verify edits in both multifile runbooks
        let deploy_main_uri = Url::from_file_path(deploy_dir.join("main.tx")).unwrap();
        let test_setup_uri = Url::from_file_path(test_dir.join("setup.tx")).unwrap();
        let test_run_uri = Url::from_file_path(test_dir.join("run.tx")).unwrap();

        assert_edits(&changes, &manifest_uri, 1, "auth_token", "manifest");
        assert_edits(&changes, &deploy_main_uri, 1, "auth_token", "deploy/main.tx");
        assert_edits(&changes, &test_setup_uri, 1, "auth_token", "test/setup.tx");
        assert_edits(&changes, &test_run_uri, 1, "auth_token", "test/run.tx");
    }

    /// Tests renaming an input when the multifile runbook is in a nested directory structure.
    ///
    /// This test verifies that the rename operation correctly discovers and updates
    /// files in multifile runbooks that are located in deeply nested paths
    /// (e.g., `./runbooks/production/deploy`).
    #[test]
    fn test_rename_input_in_nested_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        let manifest_content = r#"
runbooks:
  - name: complex
    location: ./runbooks/production/deploy

environments:
  global:
    region: "us-east-1"
"#;

        let (workspace_state, handler, manifest_uri) =
            setup_workspace(manifest_content, workspace_root);

        // Create nested directory structure for multifile runbook
        let runbook_dir = workspace_root.join("runbooks/production/deploy");
        create_runbook_files(
            &runbook_dir,
            &[
                (
                    "config.tx",
                    r#"
variable "aws_region" {
    value = input.region
}
"#,
                ),
                (
                    "actions.tx",
                    r#"
action "deploy" "aws::deploy" {
    region = input.region
}
"#,
                ),
            ],
        );

        // Rename "region" to "aws_region" from manifest
        // Line 7 is "    region: "us-east-1"" (line 0 is blank from r#")
        let params = create_rename_params(manifest_uri.clone(), 7, 4, "aws_region");

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Verify files in nested directories are discovered and updated
        let config_uri = Url::from_file_path(runbook_dir.join("config.tx")).unwrap();
        let actions_uri = Url::from_file_path(runbook_dir.join("actions.tx")).unwrap();

        assert_edits(&changes, &manifest_uri, 1, "aws_region", "manifest");
        assert_edits(&changes, &config_uri, 1, "aws_region", "nested config.tx");
        assert_edits(&changes, &actions_uri, 1, "aws_region", "nested actions.tx");
    }
}
