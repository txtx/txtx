//! Test for renaming inputs in manifest YAML across all environments

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::RenameHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rename_input_in_manifest_all_environments() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with inputs defined in multiple environments
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: main.tx

environments:
  global:
    confirmations: 12
    timeout: 30
  sepolia:
    confirmations: 6
    timeout: 15
  mainnet:
    confirmations: 20
    timeout: 60
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create main.tx that uses input.confirmations
        let main_content = r#"
action "deploy" "evm::deploy_contract" {
    wait_blocks = input.confirmations
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        // Open main.tx
        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Open manifest so workspace knows about it
        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // Rename "confirmations" to "wait_for" by clicking on it in "input.confirmations"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: main_uri.clone() },
                position: Position { line: 2, character: 25 }, // On "confirmations" in "input.confirmations"
            },
            new_name: "wait_for".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Should have edits for both main.tx and txtx.yml
        assert!(changes.contains_key(&main_uri), "Should rename in main.tx");
        assert!(changes.contains_key(&manifest_uri), "Should rename in manifest");

        // Check main.tx edit
        let main_edits = &changes[&main_uri];
        assert_eq!(main_edits.len(), 1, "Should have 1 edit in main.tx");
        assert_eq!(main_edits[0].new_text, "wait_for");

        // Check manifest edits - should have 3 edits (one per environment)
        let manifest_edits = &changes[&manifest_uri];
        assert_eq!(manifest_edits.len(), 3,
            "Should have 3 edits in manifest (global, sepolia, mainnet)");

        for edit in manifest_edits {
            assert_eq!(edit.new_text, "wait_for",
                "All manifest edits should replace with 'wait_for'");
        }

        // Verify that the edits are for the "confirmations" key in YAML
        // Apply edits and check result contains the new key name
        let mut result_content = manifest_content.to_string();
        let mut edits_sorted = manifest_edits.clone();
        edits_sorted.sort_by(|a, b| {
            b.range.start.line.cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });

        for edit in edits_sorted {
            let lines: Vec<&str> = result_content.lines().collect();
            let line_idx = edit.range.start.line as usize;
            let line = lines[line_idx];
            let start = edit.range.start.character as usize;
            let end = edit.range.end.character as usize;

            let new_line = format!("{}{}{}",
                &line[..start],
                &edit.new_text,
                &line[end..]);

            let mut new_lines = lines.clone();
            new_lines[line_idx] = &new_line;
            result_content = new_lines.join("\n");
        }

        // Verify all three environments now have "wait_for" instead of "confirmations"
        assert!(result_content.contains("wait_for: 12"),
            "Should rename in global environment");
        assert!(result_content.contains("wait_for: 6"),
            "Should rename in sepolia environment");
        assert!(result_content.contains("wait_for: 20"),
            "Should rename in mainnet environment");

        // Original key should not exist anymore
        assert!(!result_content.contains("confirmations:"),
            "Original 'confirmations' key should be replaced");
    }

    #[test]
    fn test_rename_input_includes_closed_runbooks() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with multiple runbooks
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy.tx
  - name: config
    location: config.tx

environments:
  global:
    api_key: default_key
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create deploy.tx (will be opened)
        let deploy_content = r#"
action "call_api" "http::get" {
    auth = input.api_key
}
"#;
        fs::write(workspace_root.join("deploy.tx"), deploy_content).unwrap();

        // Create config.tx (will NOT be opened - closed file)
        let config_content = r#"
variable "api_endpoint" {
    value = "https://api.example.com/${input.api_key}"
}
"#;
        fs::write(workspace_root.join("config.tx"), config_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        // Open deploy.tx and manifest
        let deploy_uri = Url::from_file_path(workspace_root.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_uri.clone(), deploy_content.to_string());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // NOTE: config.tx is NOT opened - it's a closed file

        // Rename "api_key" to "auth_token"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 2, character: 18 }, // On "api_key" in "input.api_key"
            },
            new_name: "auth_token".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Should have edits for deploy.tx, manifest, AND config.tx (even though closed)
        let config_uri = Url::from_file_path(workspace_root.join("config.tx")).unwrap();

        assert!(changes.contains_key(&deploy_uri), "Should rename in deploy.tx (open)");
        assert!(changes.contains_key(&manifest_uri), "Should rename in manifest");
        assert!(changes.contains_key(&config_uri),
            "Should rename in config.tx even though it's not open");

        // Verify config.tx edit
        let config_edits = &changes[&config_uri];
        assert_eq!(config_edits.len(), 1, "Should have 1 edit in closed config.tx");
        assert_eq!(config_edits[0].new_text, "auth_token");
    }
}
