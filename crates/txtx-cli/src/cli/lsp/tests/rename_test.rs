//! Tests for rename with multi-environment support

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::RenameHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rename_variable_across_all_environments() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create variable definition
        let variables_content = r#"
variable "api_key" {
    value = "default_key"
}
"#;
        fs::write(workspace_root.join("variables.tx"), variables_content).unwrap();

        // Create config.sepolia.tx with reference
        let config_sepolia = r#"
action "setup" "evm::call" {
    key = variable.api_key
}
"#;
        fs::write(workspace_root.join("config.sepolia.tx"), config_sepolia).unwrap();

        // Create config.mainnet.tx with reference
        let config_mainnet = r#"
action "setup" "evm::call" {
    key = variable.api_key
}
"#;
        fs::write(workspace_root.join("config.mainnet.tx"), config_mainnet).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        workspace_state.write().set_current_environment(Some("sepolia".to_string()));

        let handler = RenameHandler::new(workspace_state.clone());

        // Open documents
        let variables_uri = Url::from_file_path(workspace_root.join("variables.tx")).unwrap();
        workspace_state.write().open_document(variables_uri.clone(), variables_content.to_string());

        let config_sepolia_uri = Url::from_file_path(workspace_root.join("config.sepolia.tx")).unwrap();
        workspace_state.write().open_document(config_sepolia_uri.clone(), config_sepolia.to_string());

        let config_mainnet_uri = Url::from_file_path(workspace_root.join("config.mainnet.tx")).unwrap();
        workspace_state.write().open_document(config_mainnet_uri.clone(), config_mainnet.to_string());

        // Rename "api_key" to "auth_key"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: variables_uri.clone() },
                position: Position { line: 1, character: 10 }, // On "api_key"
            },
            new_name: "auth_key".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");

        // Verify we have changes for all files
        let changes = workspace_edit.changes.expect("Should have changes");

        assert!(
            changes.contains_key(&variables_uri),
            "Should have edits for variables.tx"
        );
        assert!(
            changes.contains_key(&config_sepolia_uri),
            "Should have edits for config.sepolia.tx"
        );
        assert!(
            changes.contains_key(&config_mainnet_uri),
            "Should have edits for config.mainnet.tx (even though it's not current env)"
        );

        // Verify the edits in variables.tx
        let var_edits = &changes[&variables_uri];
        assert_eq!(var_edits.len(), 1, "Should have 1 edit in variables.tx");
        assert_eq!(var_edits[0].new_text, "auth_key");

        // Verify the edits in both config files
        let sepolia_edits = &changes[&config_sepolia_uri];
        assert_eq!(sepolia_edits.len(), 1, "Should have 1 edit in config.sepolia.tx");
        assert_eq!(sepolia_edits[0].new_text, "auth_key");

        let mainnet_edits = &changes[&config_mainnet_uri];
        assert_eq!(mainnet_edits.len(), 1, "Should have 1 edit in config.mainnet.tx");
        assert_eq!(mainnet_edits[0].new_text, "auth_key");
    }

    #[test]
    fn test_rename_handles_both_long_and_short_forms() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create content with both var. and variable. forms
        let content = r#"
variable "count" {
    value = 10
}

action "test1" "evm::call" {
    num = variable.count
}

action "test2" "evm::call" {
    num = var.count
}
"#;
        fs::write(workspace_root.join("main.tx"), content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = RenameHandler::new(workspace_state.clone());

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), content.to_string());

        // Rename "count" to "total"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: main_uri.clone() },
                position: Position { line: 1, character: 10 }, // On "count" in definition
            },
            new_name: "total".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");
        let edits = &changes[&main_uri];

        // Should rename:
        // 1. variable "count" definition
        // 2. variable.count reference
        // 3. var.count reference
        assert_eq!(edits.len(), 3, "Should have 3 edits (definition + 2 references)");

        // All edits should change to "total"
        for edit in edits {
            assert_eq!(edit.new_text, "total");
        }
    }

    #[test]
    fn test_rename_signer_across_environments() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create signer definitions in different environments
        let signers_sepolia = r#"
signer "operator" "evm::web_wallet" {
    expected_address = input.sepolia_operator
}
"#;
        fs::write(workspace_root.join("signers.sepolia.tx"), signers_sepolia).unwrap();

        let signers_mainnet = r#"
signer "operator" "evm::web_wallet" {
    expected_address = input.mainnet_operator
}
"#;
        fs::write(workspace_root.join("signers.mainnet.tx"), signers_mainnet).unwrap();

        // Create usage
        let main_content = r#"
action "approve" "evm::call" {
    signer = signer.operator
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        workspace_state.write().set_current_environment(Some("sepolia".to_string()));

        let handler = RenameHandler::new(workspace_state.clone());

        // Open documents
        let signers_sepolia_uri = Url::from_file_path(workspace_root.join("signers.sepolia.tx")).unwrap();
        workspace_state.write().open_document(signers_sepolia_uri.clone(), signers_sepolia.to_string());

        let signers_mainnet_uri = Url::from_file_path(workspace_root.join("signers.mainnet.tx")).unwrap();
        workspace_state.write().open_document(signers_mainnet_uri.clone(), signers_mainnet.to_string());

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Rename "operator" to "deployer"
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: signers_sepolia_uri.clone() },
                position: Position { line: 1, character: 10 }, // On "operator"
            },
            new_name: "deployer".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let workspace_edit = handler.rename(params).expect("Should return workspace edit");
        let changes = workspace_edit.changes.expect("Should have changes");

        // Should rename in ALL environment files
        assert!(
            changes.contains_key(&signers_sepolia_uri),
            "Should rename in signers.sepolia.tx"
        );
        assert!(
            changes.contains_key(&signers_mainnet_uri),
            "Should rename in signers.mainnet.tx (even though not current env)"
        );
        assert!(
            changes.contains_key(&main_uri),
            "Should rename usage in main.tx"
        );
    }
}
