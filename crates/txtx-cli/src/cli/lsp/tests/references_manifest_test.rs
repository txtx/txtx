//! Test for finding references to inputs in manifest YAML and all runbooks

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::ReferencesHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams, ReferenceContext};
    use std::collections::HashSet;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_input_references_in_manifest_all_environments() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with input defined in multiple environments
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: main.tx

environments:
  global:
    confirmations: 12
  sepolia:
    confirmations: 6
  mainnet:
    confirmations: 20
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
        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open main.tx
        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Open manifest
        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // Find references to "confirmations" from "input.confirmations"
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: main_uri.clone() },
                position: Position { line: 2, character: 25 }, // On "confirmations" in "input.confirmations"
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
        };

        let locations = handler.find_references(params)
            .expect("Should find references");

        // Should find references in both main.tx and manifest (3 environments)
        // Total: 1 (main.tx) + 3 (manifest: global, sepolia, mainnet) = 4
        assert!(locations.len() >= 4,
            "Should find at least 4 references (1 in main.tx + 3 in manifest), found {}",
            locations.len());

        // Verify we have references from both files
        let file_paths: HashSet<String> = locations.iter()
            .map(|loc| loc.uri.to_file_path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_paths.iter().any(|p| p.ends_with("main.tx")),
            "Should find reference in main.tx");
        assert!(file_paths.iter().any(|p| p.ends_with("txtx.yml")),
            "Should find references in manifest");

        // Count manifest references
        let manifest_refs = locations.iter()
            .filter(|loc| loc.uri == manifest_uri)
            .count();

        assert_eq!(manifest_refs, 3,
            "Should find 3 references in manifest (one per environment)");
    }

    #[test]
    fn test_find_input_references_includes_closed_runbooks() {
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
        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open deploy.tx and manifest
        let deploy_uri = Url::from_file_path(workspace_root.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_uri.clone(), deploy_content.to_string());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // NOTE: config.tx is NOT opened - it's a closed file

        // Find references to "api_key" from "input.api_key"
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 2, character: 18 }, // On "api_key" in "input.api_key"
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
        };

        let locations = handler.find_references(params)
            .expect("Should find references");

        // Should find references in deploy.tx, manifest, AND config.tx (even though closed)
        // Total: 1 (deploy.tx) + 1 (manifest global) + 1 (config.tx) = 3
        assert!(locations.len() >= 3,
            "Should find at least 3 references (deploy.tx + manifest + closed config.tx), found {}",
            locations.len());

        let file_paths: HashSet<String> = locations.iter()
            .map(|loc| loc.uri.to_file_path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_paths.iter().any(|p| p.ends_with("deploy.tx")),
            "Should find reference in deploy.tx (open)");
        assert!(file_paths.iter().any(|p| p.ends_with("txtx.yml")),
            "Should find reference in manifest");
        assert!(file_paths.iter().any(|p| p.ends_with("config.tx")),
            "Should find reference in config.tx even though it's not open");

        // Verify config.tx reference
        let config_uri = Url::from_file_path(workspace_root.join("config.tx")).unwrap();
        let config_refs = locations.iter()
            .filter(|loc| loc.uri == config_uri)
            .count();

        assert_eq!(config_refs, 1,
            "Should find 1 reference in closed config.tx");
    }
}
