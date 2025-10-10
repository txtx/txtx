//! Go-to-definition handler with multi-file support
//!
//! This handler supports:
//! - input references to manifest environments
//! - flow references to flows.tx
//! - var references within the same file
//! - action references within the same file

use crate::cli::lsp::hcl_ast::{self, Reference};
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;
use regex::Regex;
use std::path::PathBuf;

#[derive(Clone)]
pub struct DefinitionHandler {
    workspace: SharedWorkspaceState,
}

impl DefinitionHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        eprintln!("[Definition] Request for {:?} at {}:{}", uri, position.line, position.character);

        let workspace = self.workspace.read();
        let document = workspace.get_document(uri)?;
        let content = document.content();

        // Extract the reference at cursor position
        let reference = extract_reference_at_position(content, &position)?;
        eprintln!("[Definition] Found reference: {:?}", reference);

        match reference {
            Reference::Input(var_name) => {
                // Look for input in manifest environments
                if let Some(manifest) = workspace.get_manifest_for_document(uri) {
                    if let Some(location) = find_input_in_manifest(&manifest.uri, &var_name) {
                        eprintln!("[Definition] Found input '{}' in manifest", var_name);
                        return Some(GotoDefinitionResponse::Scalar(location));
                    }
                }
            }
            Reference::Flow(flow_name) => {
                // Look for flow definition in flows.tx
                if let Some(location) = find_flow_definition(uri, &flow_name) {
                    eprintln!("[Definition] Found flow '{}' definition", flow_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
            Reference::FlowField(field_name) => {
                drop(workspace);
                let locations = find_flows_with_field(uri, &field_name, &self.workspace);

                if locations.is_empty() {
                    eprintln!("[Definition] No flows found with field '{}'", field_name);
                    return None;
                } else if locations.len() == 1 {
                    eprintln!("[Definition] Found 1 flow with field '{}'", field_name);
                    return Some(GotoDefinitionResponse::Scalar(locations.into_iter().next()?));
                } else {
                    eprintln!("[Definition] Found {} flows with field '{}'", locations.len(), field_name);
                    eprintln!("[Definition] Returning Array response with locations:");
                    for (i, loc) in locations.iter().enumerate() {
                        eprintln!("[Definition]   [{}] {}:{}:{}", i, loc.uri.path(), loc.range.start.line, loc.range.start.character);
                    }
                    return Some(GotoDefinitionResponse::Array(locations));
                }
            }
            Reference::Variable(var_name) => {
                // Look for variable definition in current file
                if let Some(location) = find_variable_definition(uri, content, &var_name) {
                    eprintln!("[Definition] Found variable '{}' definition", var_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
            Reference::Action(action_name) => {
                // Look for action definition in current file
                if let Some(location) = find_action_definition(uri, content, &action_name) {
                    eprintln!("[Definition] Found action '{}' definition", action_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
            Reference::Signer(signer_name) => {
                // Look for signer definition in current file or environment-specific files
                if let Some(location) = find_signer_definition(uri, content, &signer_name) {
                    eprintln!("[Definition] Found signer '{}' definition", signer_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }

                // Check environment-specific files using workspace environment
                let workspace_env = workspace.get_current_environment();
                if let Some(location) = find_signer_in_environment_files(uri, &signer_name, workspace_env.as_deref()) {
                    eprintln!("[Definition] Found signer '{}' in environment file", signer_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
            Reference::Output(_) => {
                // Output references don't have definitions to navigate to
                eprintln!("[Definition] Output references not supported");
            }
        }

        eprintln!("[Definition] No definition found");
        None
    }
}

fn extract_reference_at_position(content: &str, position: &Position) -> Option<Reference> {
    let line = content.lines().nth(position.line as usize)?;

    // Special case: Check for signer reference in signer = "name" format
    // This is a string literal pattern that AST won't detect as a reference
    let signer_string_re = Regex::new(r#"signer\s*=\s*"([^"]+)""#).ok()?;
    for capture in signer_string_re.captures_iter(line) {
        if let Some(name_match) = capture.get(1) {
            let name_range = (name_match.start() as u32)..(name_match.end() as u32);

            // Check if cursor is within the name part specifically (exclusive end)
            if name_range.contains(&position.character) {
                return Some(Reference::Signer(name_match.as_str().to_string()));
            }
        }
    }

    // Use lenient AST-based extraction (includes regex fallback for better UX)
    let (reference, _range) = hcl_ast::extract_reference_at_position_lenient(content, *position)?;

    // Filter out Output references (not supported for go-to-definition)
    match reference {
        Reference::Output(_) => None,
        _ => Some(reference),
    }
}

fn find_input_in_manifest(manifest_uri: &Url, var_name: &str) -> Option<Location> {
    if let Ok(content) = std::fs::read_to_string(manifest_uri.path()) {
        for (line_num, line) in content.lines().enumerate() {
            // Look for the variable in environments section
            if line.trim_start().starts_with(&format!("{}:", var_name)) {
                return Some(Location {
                    uri: manifest_uri.clone(),
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                });
            }
        }
    }
    None
}

fn find_flow_definition(current_uri: &Url, flow_name: &str) -> Option<Location> {
    // Construct path to flows.tx in the same directory
    let current_path = PathBuf::from(current_uri.path());
    if let Some(dir) = current_path.parent() {
        let flows_path = dir.join("flows.tx");

        if flows_path.exists() {
            if let Ok(flows_uri) = Url::from_file_path(&flows_path) {
                if let Ok(content) = std::fs::read_to_string(&flows_path) {
                    // Look for flow definition
                    let pattern = format!(r#"flow\s+"{}"\s*\{{"#, flow_name);
                    if let Ok(re) = Regex::new(&pattern) {
                        for (line_num, line) in content.lines().enumerate() {
                            if re.is_match(line) {
                                return Some(Location {
                                    uri: flows_uri,
                                    range: Range {
                                        start: Position { line: line_num as u32, character: 0 },
                                        end: Position {
                                            line: line_num as u32,
                                            character: line.len() as u32,
                                        },
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn find_variable_definition(uri: &Url, content: &str, var_name: &str) -> Option<Location> {
    // Look for variable definition pattern
    let pattern = format!(r#"variable\s+"{}"\s*\{{"#, var_name);
    if let Ok(re) = Regex::new(&pattern) {
        for (line_num, line) in content.lines().enumerate() {
            if re.is_match(line) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                });
            }
        }
    }
    None
}

fn find_action_definition(uri: &Url, content: &str, action_name: &str) -> Option<Location> {
    // Look for action definition pattern
    let pattern = format!(r#"action\s+"{}"\s+"[^"]+"\s*\{{"#, action_name);
    if let Ok(re) = Regex::new(&pattern) {
        for (line_num, line) in content.lines().enumerate() {
            if re.is_match(line) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                });
            }
        }
    }
    None
}

fn find_signer_definition(uri: &Url, content: &str, signer_name: &str) -> Option<Location> {
    // Look for signer definition pattern: signer "name" "type" {
    let pattern = format!(r#"signer\s+"{}"\s+"[^"]+"\s*\{{"#, regex::escape(signer_name));
    if let Ok(re) = Regex::new(&pattern) {
        for (line_num, line) in content.lines().enumerate() {
            if re.is_match(line) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_signer_reference_from_string() {
        let content = r#"action "test" "evm::send_tx" {
  signer = "my_signer"
}"#;
        // Line 1 is: '  signer = "my_signer"'
        // "my_signer" starts at position 12

        // Test cursor on "my_signer" (the 'm' at position 12)
        let position = Position { line: 1, character: 12 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Signer(ref name)) if name == "my_signer"));

        // Test cursor at the end of "my_signer" (position 20)
        let position = Position { line: 1, character: 20 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Signer(ref name)) if name == "my_signer"));

        // Test cursor outside the name (position 22, after closing quote)
        let position = Position { line: 1, character: 22 };
        let result = extract_reference_at_position(content, &position);
        assert!(result.is_none() || !matches!(result, Some(Reference::Signer(_))));
    }

    #[test]
    fn test_extract_signer_reference_from_property() {
        let content = "  signer = signer.my_signer";

        // Test cursor on "signer.my_signer"
        let position = Position { line: 0, character: 15 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Signer(ref name)) if name == "my_signer"));
    }

    #[test]
    fn test_extract_variable_reference_full_form() {
        let content = "value = variable.my_var + 1";

        // Test cursor on "variable.my_var"
        let position = Position { line: 0, character: 12 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Variable(ref name)) if name == "my_var"));
    }

    #[test]
    fn test_extract_variable_reference_short_form() {
        let content = "value = var.count * 2";

        // Test cursor on "var.count"
        let position = Position { line: 0, character: 10 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Variable(ref name)) if name == "count"));
    }

    #[test]
    fn test_extract_variable_from_definition() {
        let content = r#"variable "api_key" {"#;

        // Test cursor on "api_key" in the definition
        let position = Position { line: 0, character: 12 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::Variable(ref name)) if name == "api_key"));
    }

    #[test]
    fn test_find_variable_definition() {
        let content = r#"
variable "count" {
    value = 10
}

variable "api_key" {
    value = "secret"
}
"#;
        let uri = Url::parse("file:///test.tx").unwrap();

        // Test finding "count" variable
        let location = find_variable_definition(&uri, content, "count");
        assert!(location.is_some());
        if let Some(loc) = location {
            assert_eq!(loc.range.start.line, 1);
        }

        // Test finding "api_key" variable
        let location = find_variable_definition(&uri, content, "api_key");
        assert!(location.is_some());
        if let Some(loc) = location {
            assert_eq!(loc.range.start.line, 5);
        }

        // Test non-existent variable
        let location = find_variable_definition(&uri, content, "nonexistent");
        assert!(location.is_none());
    }

    #[test]
    fn test_find_signer_with_workspace_environment() {
        use crate::cli::lsp::workspace::SharedWorkspaceState;
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory with test files
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create main.tx (no environment in filename)
        let main_tx_path = temp_path.join("main.tx");
        fs::write(&main_tx_path, r#"
action "approve_tokens" "evm::call_contract" {
    signer = signer.operator
}
"#).unwrap();

        // Create signers.sepolia.tx (environment-specific signer file)
        let signers_sepolia_path = temp_path.join("signers.sepolia.tx");
        fs::write(&signers_sepolia_path, r#"
signer "operator" "evm::web_wallet" {
    expected_address = input.sepolia_operator
}
"#).unwrap();

        // Create signers.mainnet.tx (different environment, should NOT be selected)
        let signers_mainnet_path = temp_path.join("signers.mainnet.tx");
        fs::write(&signers_mainnet_path, r#"
signer "operator" "evm::web_wallet" {
    expected_address = input.mainnet_operator
}
"#).unwrap();

        // Create workspace with environment set to "sepolia"
        let workspace_state = SharedWorkspaceState::new();
        workspace_state.write().set_current_environment(Some("sepolia".to_string()));

        // Create handler
        let handler = DefinitionHandler::new(workspace_state.clone());

        // Open main.tx in workspace
        let main_uri = Url::from_file_path(&main_tx_path).unwrap();
        workspace_state.write().open_document(
            main_uri.clone(),
            fs::read_to_string(&main_tx_path).unwrap(),
        );

        // Test goto definition on "signer.operator" in main.tx
        // Line 2, character 21 is on "operator" in "signer = signer.operator"
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: main_uri.clone() },
                position: Position { line: 2, character: 21 },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        // This should find the definition in signers.sepolia.tx
        let result = handler.goto_definition(params);

        assert!(result.is_some(), "Should find signer definition in environment-specific file");

        if let Some(GotoDefinitionResponse::Scalar(location)) = result {
            // Verify it points to signers.sepolia.tx
            assert!(location.uri.path().ends_with("signers.sepolia.tx"),
                    "Should resolve to signers.sepolia.tx, got: {}", location.uri.path());
            // Verify it points to the signer definition line
            assert_eq!(location.range.start.line, 1, "Should point to signer definition line");
        } else {
            panic!("Expected scalar location response");
        }
    }

    #[test]
    fn test_extract_flow_field_reference() {
        let content = "value = flow.chain_id";

        // Test cursor on "chain_id" in "flow.chain_id"
        let position = Position { line: 0, character: 13 };
        let result = extract_reference_at_position(content, &position);
        assert!(matches!(result, Some(Reference::FlowField(ref name)) if name == "chain_id"));
    }

    #[test]
    fn test_find_flows_in_content_single_match() {
        let content = r#"
flow "super1" {
  chain_id = "11155111"
}

flow "super2" {
  network = "sepolia"
}
"#;
        let uri = Url::parse("file:///flows.tx").unwrap();

        // Test finding flows with "chain_id" field
        let locations = find_flows_in_content(content, "chain_id", &uri);
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].range.start.line, 1); // flow "super1" is on line 1
    }

    #[test]
    fn test_find_flows_in_content_multiple_matches() {
        let content = r#"
flow "super1" {
  chain_id = "11155111"
}

flow "super2" {
  chain_id = "2"
}

flow "super3" {
  chain_id = "3"
}
"#;
        let uri = Url::parse("file:///flows.tx").unwrap();

        // Test finding flows with "chain_id" field
        let locations = find_flows_in_content(content, "chain_id", &uri);
        assert_eq!(locations.len(), 3);
        assert_eq!(locations[0].range.start.line, 1); // flow "super1"
        assert_eq!(locations[1].range.start.line, 5); // flow "super2"
        assert_eq!(locations[2].range.start.line, 9); // flow "super3"
    }

    #[test]
    fn test_find_flows_in_content_no_match() {
        let content = r#"
flow "super1" {
  chain_id = "11155111"
}

flow "super2" {
  network = "sepolia"
}
"#;
        let uri = Url::parse("file:///flows.tx").unwrap();

        // Test finding flows with non-existent field
        let locations = find_flows_in_content(content, "nonexistent", &uri);
        assert_eq!(locations.len(), 0);
    }

    #[test]
    fn test_search_flow_block_field_found() {
        let lines = vec![
            "flow \"super1\" {",
            "  chain_id = \"11155111\"",
            "  network = \"sepolia\"",
            "}",
        ];

        let field_re = Regex::new(r"^\s*chain_id\s*=").unwrap();
        let result = search_flow_block(&lines, 0, &field_re);
        assert!(result.is_some());
    }

    #[test]
    fn test_search_flow_block_field_not_found() {
        let lines = vec![
            "flow \"super1\" {",
            "  network = \"sepolia\"",
            "}",
        ];

        let field_re = Regex::new(r"^\s*chain_id\s*=").unwrap();
        let result = search_flow_block(&lines, 0, &field_re);
        assert!(result.is_none());
    }

    #[test]
    fn test_search_flow_block_nested_braces() {
        let lines = vec![
            "flow \"super1\" {",
            "  config {",
            "    chain_id = \"11155111\"",
            "  }",
            "}",
        ];

        let field_re = Regex::new(r"^\s*chain_id\s*=").unwrap();
        let result = search_flow_block(&lines, 0, &field_re);
        assert!(result.is_some());
    }

    #[test]
    fn test_flow_field_goto_definition_single_flow() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create flows.tx with one flow
        let flows_tx_path = temp_path.join("flows.tx");
        fs::write(&flows_tx_path, r#"
flow "super1" {
  chain_id = "11155111"
}
"#).unwrap();

        // Create deploy.tx with flow.chain_id reference
        let deploy_tx_path = temp_path.join("deploy.tx");
        fs::write(&deploy_tx_path, r#"
action "deploy" "evm::deploy_contract" {
  constructor_args = [
    flow.chain_id
  ]
}
"#).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = DefinitionHandler::new(workspace_state.clone());

        let deploy_uri = Url::from_file_path(&deploy_tx_path).unwrap();
        workspace_state.write().open_document(
            deploy_uri.clone(),
            fs::read_to_string(&deploy_tx_path).unwrap(),
        );

        // Test goto definition on "chain_id" in "flow.chain_id"
        // Line 3, character 9 is on "chain_id"
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 3, character: 9 },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handler.goto_definition(params);
        assert!(result.is_some(), "Should find flow with chain_id field");

        if let Some(GotoDefinitionResponse::Scalar(location)) = result {
            assert!(location.uri.path().ends_with("flows.tx"));
            assert_eq!(location.range.start.line, 1); // flow "super1" is on line 1
        } else {
            panic!("Expected scalar location response for single flow");
        }
    }

    #[test]
    fn test_flow_field_goto_definition_multiple_flows() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create flows.tx with multiple flows
        let flows_tx_path = temp_path.join("flows.tx");
        fs::write(&flows_tx_path, r#"
flow "super1" {
  chain_id = "11155111"
}

flow "super2" {
  chain_id = "2"
}

flow "super3" {
  chain_id = "3"
}
"#).unwrap();

        // Create deploy.tx with flow.chain_id reference
        let deploy_tx_path = temp_path.join("deploy.tx");
        fs::write(&deploy_tx_path, r#"
action "deploy" "evm::deploy_contract" {
  constructor_args = [
    flow.chain_id
  ]
}
"#).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = DefinitionHandler::new(workspace_state.clone());

        let deploy_uri = Url::from_file_path(&deploy_tx_path).unwrap();
        workspace_state.write().open_document(
            deploy_uri.clone(),
            fs::read_to_string(&deploy_tx_path).unwrap(),
        );

        // Test goto definition on "chain_id" in "flow.chain_id"
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 3, character: 9 },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handler.goto_definition(params);
        assert!(result.is_some(), "Should find multiple flows with chain_id field");

        if let Some(GotoDefinitionResponse::Array(locations)) = result {
            assert_eq!(locations.len(), 3);
            assert!(locations[0].uri.path().ends_with("flows.tx"));
            assert_eq!(locations[0].range.start.line, 1); // flow "super1"
            assert_eq!(locations[1].range.start.line, 5); // flow "super2"
            assert_eq!(locations[2].range.start.line, 9); // flow "super3"
        } else {
            panic!("Expected array location response for multiple flows");
        }
    }

    #[test]
    fn test_flow_field_goto_definition_no_match() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create flows.tx with flows that don't have the field
        let flows_tx_path = temp_path.join("flows.tx");
        fs::write(&flows_tx_path, r#"
flow "super1" {
  network = "sepolia"
}
"#).unwrap();

        // Create deploy.tx with flow.chain_id reference
        let deploy_tx_path = temp_path.join("deploy.tx");
        fs::write(&deploy_tx_path, r#"
action "deploy" "evm::deploy_contract" {
  constructor_args = [
    flow.chain_id
  ]
}
"#).unwrap();

        let workspace_state = SharedWorkspaceState::new();
        let handler = DefinitionHandler::new(workspace_state.clone());

        let deploy_uri = Url::from_file_path(&deploy_tx_path).unwrap();
        workspace_state.write().open_document(
            deploy_uri.clone(),
            fs::read_to_string(&deploy_tx_path).unwrap(),
        );

        // Test goto definition on "chain_id" in "flow.chain_id"
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: deploy_uri.clone() },
                position: Position { line: 3, character: 9 },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handler.goto_definition(params);
        assert!(result.is_none(), "Should not find any flows with chain_id field");
    }
}

/// Searches for signer in environment-appropriate files.
///
/// Only includes files matching the workspace environment or files without environment markers.
/// Excludes files from other environments to prevent incorrect resolution.
fn find_signer_in_environment_files(uri: &Url, signer_name: &str, workspace_env: Option<&str>) -> Option<Location> {
    use crate::cli::lsp::utils::environment::extract_environment_from_path;

    let current_path = uri.to_file_path().ok()?;
    let dir = current_path.parent()?;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !path.extension().map_or(false, |e| e == "tx") {
                continue;
            }

            // Extract environment from filename
            let file_env = extract_environment_from_path(&path);

            // Only include file if:
            // 1. It has no environment marker (e.g., signers.tx), OR
            // 2. Its environment matches the workspace environment
            let should_include = match (file_env.as_deref(), workspace_env) {
                (None, _) => true,                           // No env marker - always include
                (Some(file_e), Some(work_e)) => file_e == work_e, // Env matches
                (Some(_), None) => false,                    // File has env but workspace doesn't - exclude
            };

            if should_include {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(file_uri) = Url::from_file_path(&path) {
                        if let Some(location) = find_signer_definition(&file_uri, &content, signer_name) {
                            return Some(location);
                        }
                    }
                }
            }
        }
    }

    None
}

// Cached regexes for flow field search
lazy_static::lazy_static! {
    static ref FLOW_RE: Regex = Regex::new(r#"flow\s+"([^"]+)"\s*\{"#).expect("valid flow regex");
}

/// Find all flows that define a specific field
fn find_flows_with_field(
    current_uri: &Url,
    field_name: &str,
    workspace: &SharedWorkspaceState,
) -> Vec<Location> {
    let files_to_search = {
        let current_path = current_uri.to_file_path().ok();
        if let Some(path) = current_path {
            if let Some(dir) = path.parent() {
                get_directory_tx_files(dir)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    eprintln!("[Definition] Searching {} files for field '{}'", files_to_search.len(), field_name);
    for file in &files_to_search {
        eprintln!("[Definition]   - {}", file.path());
    }

    let locations: Vec<Location> = files_to_search
        .into_iter()
        .filter_map(|file_uri| {
            file_uri
                .to_file_path()
                .ok()
                .and_then(|p| std::fs::read_to_string(&p).ok())
                .map(|content| {
                    let locs = find_flows_in_content(&content, field_name, &file_uri);
                    eprintln!("[Definition]   Found {} flows in {}", locs.len(), file_uri.path());
                    locs
                })
        })
        .flatten()
        .collect();

    eprintln!("[Definition] Total locations found: {}", locations.len());
    locations
}

/// Find all flow definitions in content that have the specified field
fn find_flows_in_content(content: &str, field_name: &str, uri: &Url) -> Vec<Location> {
    let field_pattern = format!(r"^\s*{}\s*=", regex::escape(field_name));
    let field_re = match Regex::new(&field_pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("[Definition] Failed to compile field regex: {}", e);
            return Vec::new();
        }
    };

    let lines: Vec<&str> = content.lines().collect();

    lines
        .iter()
        .enumerate()
        .filter_map(|(line_num, line)| {
            FLOW_RE.captures(line).map(|cap| {
                let flow_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                (line_num, flow_name)
            })
        })
        .filter_map(|(flow_line, _flow_name)| {
            search_flow_block(&lines, flow_line, &field_re).map(|_| Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: flow_line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: flow_line as u32,
                        character: lines[flow_line].len() as u32,
                    },
                },
            })
        })
        .collect()
}

/// Search within a flow block for a field matching the regex
fn search_flow_block(lines: &[&str], flow_line: usize, field_re: &Regex) -> Option<()> {
    let mut brace_depth = 1;
    let mut i = flow_line + 1;

    while i < lines.len() && brace_depth > 0 {
        let line = lines[i];

        brace_depth += line.matches('{').count();
        brace_depth -= line.matches('}').count();

        if field_re.is_match(line) {
            return Some(());
        }

        i += 1;
    }

    None
}

/// Get all .tx files in a directory
fn get_directory_tx_files(dir: &std::path::Path) -> Vec<Url> {
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "tx") {
                Url::from_file_path(&path).ok()
            } else {
                None
            }
        })
        .collect()
}
