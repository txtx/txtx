//! Test for renaming inputs when clicking on YAML keys in manifest file

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::RenameHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rename_input_from_yaml_key_in_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with input defined in global environment
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: main.tx

environments:
  global:
    chain_id: 11155111
    timeout: 30
  sepolia:
    chain_id: 11155111
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create main.tx that uses input.chain_id
        let main_content = r#"
action "deploy" "evm::deploy_contract" {
    chain = input.chain_id
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        // Open manifest file
        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // Open main.tx
        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Rename "chain_id" to "network_id" by clicking on the YAML key in manifest
        // Line 7 is "    chain_id: 11155111" in global environment (line 0 is blank from r#")
        // Character position 4 is at the start of "chain_id"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: manifest_uri.clone() },
                position: Position { line: 7, character: 4 }, // On "chain_id" YAML key
            },
            new_name: "network_id".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Should have edits for both manifest and main.tx
        assert!(changes.contains_key(&manifest_uri),
            "Should rename in manifest (both global and sepolia)");
        assert!(changes.contains_key(&main_uri),
            "Should rename in main.tx");

        // Check manifest edits - should have 2 edits (global and sepolia)
        let manifest_edits = &changes[&manifest_uri];
        assert_eq!(manifest_edits.len(), 2,
            "Should have 2 edits in manifest (global and sepolia environments)");

        for edit in manifest_edits {
            assert_eq!(edit.new_text, "network_id",
                "All manifest edits should replace with 'network_id'");
        }

        // Check main.tx edit - should have 1 edit
        let main_edits = &changes[&main_uri];
        assert_eq!(main_edits.len(), 1, "Should have 1 edit in main.tx");
        assert_eq!(main_edits[0].new_text, "network_id");

        // Verify the edit range in main.tx only covers "chain_id", not "input."
        let lines: Vec<&str> = main_content.lines().collect();
        let line = lines[main_edits[0].range.start.line as usize];
        let start = main_edits[0].range.start.character as usize;
        let end = main_edits[0].range.end.character as usize;
        let replaced_text = &line[start..end];

        assert_eq!(replaced_text, "chain_id",
            "Should only replace 'chain_id', not the whole reference");
    }

    #[test]
    fn test_rename_input_from_yaml_key_with_underscore() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with input that has underscores
        let manifest_content = r#"
environments:
  global:
    chain_id_xyz: 11155111
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create runbook that uses it
        let runbook_content = r#"
variable "network" {
    value = input.chain_id_xyz
}
"#;
        fs::write(workspace_root.join("config.tx"), runbook_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        let runbook_uri = Url::from_file_path(workspace_root.join("config.tx")).unwrap();
        workspace_state.write().open_document(runbook_uri.clone(), runbook_content.to_string());

        // Click on "chain_id_xyz" in the YAML key
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: manifest_uri.clone() },
                position: Position { line: 3, character: 4 }, // On "chain_id_xyz" YAML key
            },
            new_name: "network_chain_id".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        assert!(changes.contains_key(&manifest_uri), "Should rename in manifest");
        assert!(changes.contains_key(&runbook_uri), "Should rename in runbook");

        let manifest_edits = &changes[&manifest_uri];
        assert_eq!(manifest_edits.len(), 1);
        assert_eq!(manifest_edits[0].new_text, "network_chain_id");
    }
}
