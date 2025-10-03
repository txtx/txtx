//! Tests for find references with multi-environment support

#[cfg(test)]
mod tests {
    use crate::cli::lsp::handlers::ReferencesHandler;
    use crate::cli::lsp::workspace::SharedWorkspaceState;
    use lsp_types::{Position, ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams, Url};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_variable_references_across_environments() {
        // Create temp workspace
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest
        let manifest_content = r#"
environments:
  sepolia:
    description: "Sepolia testnet"
  mainnet:
    description: "Ethereum mainnet"
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create variable definition (no environment)
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

        // Create main.tx with reference (no environment)
        let main_content = r#"
action "deploy" "evm::deploy" {
    auth_key = variable.api_key
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        // Setup workspace and handler
        let workspace_state = SharedWorkspaceState::new();
        workspace_state.write().set_current_environment(Some("sepolia".to_string()));

        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open all documents
        let variables_uri = Url::from_file_path(workspace_root.join("variables.tx")).unwrap();
        workspace_state.write().open_document(variables_uri.clone(), variables_content.to_string());

        let config_sepolia_uri = Url::from_file_path(workspace_root.join("config.sepolia.tx")).unwrap();
        workspace_state.write().open_document(config_sepolia_uri.clone(), config_sepolia.to_string());

        let config_mainnet_uri = Url::from_file_path(workspace_root.join("config.mainnet.tx")).unwrap();
        workspace_state.write().open_document(config_mainnet_uri.clone(), config_mainnet.to_string());

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Find references to "api_key" from the definition
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: variables_uri.clone() },
                position: Position { line: 1, character: 10 }, // On "api_key" in variable definition
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        // Should find references in:
        // 1. variables.tx (definition)
        // 2. config.sepolia.tx (current env)
        // 3. config.mainnet.tx (other env)
        // 4. main.tx (no env)
        assert!(
            references.len() >= 3,
            "Should find at least 3 references (excluding definition). Found: {}",
            references.len()
        );

        // Verify we found references in all expected files
        let paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        assert!(
            paths.iter().any(|p| p.ends_with("config.sepolia.tx")),
            "Should find reference in config.sepolia.tx"
        );
        assert!(
            paths.iter().any(|p| p.ends_with("config.mainnet.tx")),
            "Should find reference in config.mainnet.tx"
        );
        assert!(
            paths.iter().any(|p| p.ends_with("main.tx")),
            "Should find reference in main.tx"
        );
    }

    #[test]
    fn test_find_flow_references_across_multi_file_runbook() {
        // This test reproduces the bug where references in multi-file runbooks
        // only show references in the current file, not all files in the runbook
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with multi-file runbook
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create multi-file runbook directory
        let runbook_dir = workspace_root.join("deploy");
        fs::create_dir_all(&runbook_dir).unwrap();

        // Create flows.tx with flow definition using a variable
        let flows_content = r#"
variable "network_id" {
    value = "mainnet"
}

flow "super1" {
    chain_id = variable.network_id
}

flow "super2" {
    chain_id = variable.network_id
}
"#;
        fs::write(runbook_dir.join("flows.tx"), flows_content).unwrap();

        // Create deploy.tx that also uses variable.network_id
        let deploy_content = r#"
action "deploy" "evm::deploy_contract" {
    contract = evi::get_abi_from_foundation("SimpleStorage")
    constructor_args = [
        variable.network_id
    ]
    signer = signer.deployer
}
"#;
        fs::write(runbook_dir.join("deploy.tx"), deploy_content).unwrap();

        // Setup workspace and handler
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open manifest to enable workspace discovery
        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // Open flows.tx
        let flows_uri = Url::from_file_path(runbook_dir.join("flows.tx")).unwrap();
        workspace_state.write().open_document(flows_uri.clone(), flows_content.to_string());

        // Open deploy.tx
        let deploy_uri = Url::from_file_path(runbook_dir.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_uri.clone(), deploy_content.to_string());

        // Find references to "network_id" variable from deploy.tx
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 4, character: 18 }, // On "network_id" in variable.network_id
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        println!("Found {} references:", references.len());
        for (i, reference) in references.iter().enumerate() {
            println!("  {}. {} (line {})",
                i + 1,
                reference.uri.path().split('/').last().unwrap_or(""),
                reference.range.start.line
            );
        }

        // Should find references in BOTH flows.tx AND deploy.tx
        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_flows_ref = file_paths.iter().any(|p| p.ends_with("flows.tx"));
        let has_deploy_ref = file_paths.iter().any(|p| p.ends_with("deploy.tx"));

        assert!(
            has_flows_ref,
            "Should find reference in flows.tx where network_id variable is used. Files found: {:?}",
            file_paths.iter().map(|p| p.split('/').last().unwrap_or("")).collect::<Vec<_>>()
        );

        assert!(
            has_deploy_ref,
            "Should find reference in deploy.tx where network_id variable is used. Files found: {:?}",
            file_paths.iter().map(|p| p.split('/').last().unwrap_or("")).collect::<Vec<_>>()
        );

        assert!(
            references.len() >= 3,
            "Should find at least 3 references (2 in flows.tx, 1 in deploy.tx). Found: {}",
            references.len()
        );
    }

    #[test]
    fn test_find_signer_references_across_environments() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create signers for different environments
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

        // Create main.tx that references signer
        let main_content = r#"
action "approve" "evm::call" {
    signer = signer.operator
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        workspace_state.write().set_current_environment(Some("sepolia".to_string()));

        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open documents
        let signers_sepolia_uri = Url::from_file_path(workspace_root.join("signers.sepolia.tx")).unwrap();
        workspace_state.write().open_document(signers_sepolia_uri.clone(), signers_sepolia.to_string());

        let signers_mainnet_uri = Url::from_file_path(workspace_root.join("signers.mainnet.tx")).unwrap();
        workspace_state.write().open_document(signers_mainnet_uri.clone(), signers_mainnet.to_string());

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_content.to_string());

        // Find references to "operator" signer from definition
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: signers_sepolia_uri.clone() },
                position: Position { line: 1, character: 10 }, // On "operator"
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        // Should find:
        // 1. Definition in signers.sepolia.tx (current env)
        // 2. Definition in signers.mainnet.tx (other env)
        // 3. Usage in main.tx
        assert!(
            references.len() >= 2,
            "Should find at least 2 references. Found: {}",
            references.len()
        );

        let paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        assert!(
            paths.iter().any(|p| p.ends_with("main.tx")),
            "Should find reference in main.tx"
        );
    }

    #[test]
    fn test_variable_references_scoped_to_single_runbook_only() {
        // Test that variable references are scoped to the current runbook only
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with two runbooks
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy
  - name: monitor
    location: monitor
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create deploy runbook with variable
        let deploy_dir = workspace_root.join("deploy");
        fs::create_dir_all(&deploy_dir).unwrap();

        let deploy_flows = r#"
variable "network_id" {
    value = "1"
}
"#;
        fs::write(deploy_dir.join("flows.tx"), deploy_flows).unwrap();

        let deploy_main = r#"
action "deploy" "evm::deploy" {
    network = variable.network_id
}
"#;
        fs::write(deploy_dir.join("deploy.tx"), deploy_main).unwrap();

        // Create monitor runbook with SAME variable name (different runbook)
        let monitor_dir = workspace_root.join("monitor");
        fs::create_dir_all(&monitor_dir).unwrap();

        let monitor_main = r#"
variable "network_id" {
    value = "2"
}

action "check" "evm::call" {
    network = variable.network_id
}
"#;
        fs::write(monitor_dir.join("main.tx"), monitor_main).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        // Open manifest
        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        // Open deploy files
        let deploy_flows_uri = Url::from_file_path(deploy_dir.join("flows.tx")).unwrap();
        workspace_state.write().open_document(deploy_flows_uri.clone(), deploy_flows.to_string());

        let deploy_main_uri = Url::from_file_path(deploy_dir.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_main_uri.clone(), deploy_main.to_string());

        // Open monitor files
        let monitor_main_uri = Url::from_file_path(monitor_dir.join("main.tx")).unwrap();
        workspace_state.write().open_document(monitor_main_uri.clone(), monitor_main.to_string());

        // Find references to network_id from deploy runbook
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_main_uri.clone() },
                position: Position { line: 2, character: 22 }, // On network_id in variable.network_id
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_deploy_flows = file_paths.iter().any(|p| p.contains("deploy") && p.ends_with("flows.tx"));
        let has_deploy_main = file_paths.iter().any(|p| p.contains("deploy") && p.ends_with("deploy.tx"));
        let has_monitor = file_paths.iter().any(|p| p.contains("monitor"));

        println!("Found references in files:");
        for path in &file_paths {
            println!("  - {}", path.split('/').last().unwrap_or(""));
        }

        assert!(has_deploy_flows, "Should find reference in deploy/flows.tx");
        assert!(has_deploy_main, "Should find reference in deploy/deploy.tx");
        assert!(!has_monitor, "Should NOT find reference in monitor runbook (different runbook with same variable name)");
    }

    #[test]
    fn test_flow_references_stay_within_runbook_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with two runbooks
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy
  - name: setup
    location: setup
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create deploy runbook
        let deploy_dir = workspace_root.join("deploy");
        fs::create_dir_all(&deploy_dir).unwrap();

        let deploy_flows = r#"
flow "chain_config" {
    chain_id = input.chain
}
"#;
        fs::write(deploy_dir.join("flows.tx"), deploy_flows).unwrap();

        let deploy_main = r#"
action "deploy" "evm::deploy" {
    chain = flow.chain_config
}
"#;
        fs::write(deploy_dir.join("deploy.tx"), deploy_main).unwrap();

        // Create setup runbook with SAME flow name
        let setup_dir = workspace_root.join("setup");
        fs::create_dir_all(&setup_dir).unwrap();

        let setup_flows = r#"
flow "chain_config" {
    chain_id = "hardcoded"
}
"#;
        fs::write(setup_dir.join("flows.tx"), setup_flows).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri, manifest_content.to_string());

        let deploy_flows_uri = Url::from_file_path(deploy_dir.join("flows.tx")).unwrap();
        workspace_state.write().open_document(deploy_flows_uri, deploy_flows.to_string());

        let deploy_main_uri = Url::from_file_path(deploy_dir.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_main_uri.clone(), deploy_main.to_string());

        let setup_flows_uri = Url::from_file_path(setup_dir.join("flows.tx")).unwrap();
        workspace_state.write().open_document(setup_flows_uri, setup_flows.to_string());

        // Find references from deploy runbook
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_main_uri },
                position: Position { line: 2, character: 18 }, // On chain_config in flow.chain_config
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_deploy = file_paths.iter().any(|p| p.contains("deploy"));
        let has_setup = file_paths.iter().any(|p| p.contains("setup"));

        assert!(has_deploy, "Should find references in deploy runbook");
        assert!(!has_setup, "Should NOT find references in setup runbook (different runbook)");
    }

    #[test]
    fn test_input_references_cross_all_runbooks() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with input and two runbooks
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy
  - name: monitor
    location: monitor

environments:
  global:
    api_key: "default_key"
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create deploy runbook using input
        let deploy_dir = workspace_root.join("deploy");
        fs::create_dir_all(&deploy_dir).unwrap();

        let deploy_main = r#"
action "deploy" "evm::deploy" {
    auth = input.api_key
}
"#;
        fs::write(deploy_dir.join("main.tx"), deploy_main).unwrap();

        // Create monitor runbook using same input
        let monitor_dir = workspace_root.join("monitor");
        fs::create_dir_all(&monitor_dir).unwrap();

        let monitor_main = r#"
action "check" "evm::call" {
    auth = input.api_key
}
"#;
        fs::write(monitor_dir.join("main.tx"), monitor_main).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri.clone(), manifest_content.to_string());

        let deploy_main_uri = Url::from_file_path(deploy_dir.join("main.tx")).unwrap();
        workspace_state.write().open_document(deploy_main_uri.clone(), deploy_main.to_string());

        let monitor_main_uri = Url::from_file_path(monitor_dir.join("main.tx")).unwrap();
        workspace_state.write().open_document(monitor_main_uri, monitor_main.to_string());

        // Find references to input.api_key from deploy runbook
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_main_uri },
                position: Position { line: 2, character: 17 }, // On api_key in input.api_key
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_deploy = file_paths.iter().any(|p| p.contains("deploy") && p.ends_with("main.tx"));
        let has_monitor = file_paths.iter().any(|p| p.contains("monitor") && p.ends_with("main.tx"));
        let has_manifest = file_paths.iter().any(|p| p.ends_with("txtx.yml"));

        println!("Input references found in:");
        for path in &file_paths {
            let parts: Vec<&str> = path.split('/').collect();
            let display_path = parts.iter().rev().take(2).rev().map(|s| *s).collect::<Vec<_>>().join("/");
            println!("  - {}", display_path);
        }

        assert!(has_deploy, "Should find reference in deploy/main.tx");
        assert!(has_monitor, "Should find reference in monitor/main.tx (inputs are workspace-scoped)");
        assert!(has_manifest, "Should find declaration in txtx.yml");
    }

    #[test]
    fn test_action_output_references_scoped_to_runbook() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with two runbooks
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: deploy
  - name: verify
    location: verify
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create deploy runbook
        let deploy_dir = workspace_root.join("deploy");
        fs::create_dir_all(&deploy_dir).unwrap();

        let deploy_main = r#"
action "deploy" "evm::deploy_contract" {
    contract = evi::get_abi_from_foundation("Token")
}
"#;
        fs::write(deploy_dir.join("deploy.tx"), deploy_main).unwrap();

        let deploy_output = r#"
output "contract" {
    value = action.deploy.contract_address
}
"#;
        fs::write(deploy_dir.join("output.tx"), deploy_output).unwrap();

        // Create verify runbook with SAME action name
        let verify_dir = workspace_root.join("verify");
        fs::create_dir_all(&verify_dir).unwrap();

        let verify_main = r#"
action "deploy" "evm::call_contract" {
    contract_address = input.deployed_contract
}
"#;
        fs::write(verify_dir.join("verify.tx"), verify_main).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri, manifest_content.to_string());

        let deploy_main_uri = Url::from_file_path(deploy_dir.join("deploy.tx")).unwrap();
        workspace_state.write().open_document(deploy_main_uri, deploy_main.to_string());

        let deploy_output_uri = Url::from_file_path(deploy_dir.join("output.tx")).unwrap();
        workspace_state.write().open_document(deploy_output_uri.clone(), deploy_output.to_string());

        let verify_main_uri = Url::from_file_path(verify_dir.join("verify.tx")).unwrap();
        workspace_state.write().open_document(verify_main_uri, verify_main.to_string());

        // Find references to action.deploy from output.tx
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_output_uri },
                position: Position { line: 2, character: 18 }, // On "deploy" in action.deploy
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_deploy = file_paths.iter().any(|p| p.contains("deploy"));
        let has_verify = file_paths.iter().any(|p| p.contains("verify"));

        assert!(has_deploy, "Should find references in deploy runbook");
        assert!(!has_verify, "Should NOT find references in verify runbook (different runbook with same action name)");
    }

    #[test]
    fn test_files_without_runbook_are_workspace_wide() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest WITHOUT runbooks
        let manifest_content = r#"
environments:
  global:
    description: "Default environment"
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create standalone files in workspace root (not in any runbook)
        let main_content = r#"
variable "config" {
    value = "x"
}
"#;
        fs::write(workspace_root.join("main.tx"), main_content).unwrap();

        let helper_content = r#"
action "helper" "std::print" {
    message = variable.config
}
"#;
        fs::write(workspace_root.join("helper.tx"), helper_content).unwrap();

        // Setup workspace
        let workspace_state = SharedWorkspaceState::new();
        let handler = ReferencesHandler::new(workspace_state.clone());

        let manifest_uri = Url::from_file_path(workspace_root.join("txtx.yml")).unwrap();
        workspace_state.write().open_document(manifest_uri, manifest_content.to_string());

        let main_uri = Url::from_file_path(workspace_root.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri, main_content.to_string());

        let helper_uri = Url::from_file_path(workspace_root.join("helper.tx")).unwrap();
        workspace_state.write().open_document(helper_uri.clone(), helper_content.to_string());

        // Find references to variable.config from helper.tx
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: helper_uri },
                position: Position { line: 2, character: 22 }, // On config in variable.config
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: false,
            },
        };

        let references = handler.find_references(params).expect("Should find references");

        let file_paths: Vec<String> = references.iter()
            .map(|loc| loc.uri.path().to_string())
            .collect();

        let has_main = file_paths.iter().any(|p| p.ends_with("main.tx"));
        let has_helper = file_paths.iter().any(|p| p.ends_with("helper.tx"));

        assert!(has_main, "Should find reference in main.tx");
        assert!(has_helper, "Should find reference in helper.tx");
        assert!(references.len() >= 2, "Files without runbook definition should be searched workspace-wide");
    }
}
