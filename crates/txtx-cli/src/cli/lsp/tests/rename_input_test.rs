//! Test for renaming input references correctly

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::RenameHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rename_input_preserves_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with input defined
        let manifest_content = r#"
environments:
  global:
    inputs:
      confirmations: 12
  sepolia:
    inputs:
      confirmations: 6
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

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

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
        let edits = &changes[&main_uri];

        // Should have exactly 1 edit
        assert_eq!(edits.len(), 1, "Should have 1 edit");

        // The edit should replace only "confirmations", not "input.confirmations"
        assert_eq!(edits[0].new_text, "wait_for");

        // Verify the range - should only span "confirmations", not "input."
        let edit_range = &edits[0].range;
        let lines: Vec<&str> = main_content.lines().collect();
        let line = lines[edit_range.start.line as usize];
        let start = edit_range.start.character as usize;
        let end = edit_range.end.character as usize;
        let replaced_text = &line[start..end];

        assert_eq!(replaced_text, "confirmations",
            "Should only replace 'confirmations', not the whole reference. Range: {:?}, Text: '{}'",
            edit_range, replaced_text);

        // The result should be "input.wait_for", not just "wait_for"
        let new_line = format!(
            "{}{}{}",
            &line[..start],
            &edits[0].new_text,
            &line[end..]
        );
        assert!(new_line.contains("input.wait_for"),
            "Result should be 'input.wait_for', got: '{}'", new_line);
    }
}
