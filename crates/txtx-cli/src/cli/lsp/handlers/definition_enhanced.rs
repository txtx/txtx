//! Enhanced go-to-definition handler with multi-file support
//!
//! This handler supports:
//! - input references to manifest environments
//! - flow references to flows.tx
//! - var references within the same file
//! - action references within the same file

use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;
use regex::Regex;
use std::path::PathBuf;

pub struct EnhancedDefinitionHandler {
    workspace: SharedWorkspaceState,
}

impl EnhancedDefinitionHandler {
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

                // Check environment-specific files
                if let Some(location) = find_signer_in_environment_files(uri, &signer_name) {
                    eprintln!("[Definition] Found signer '{}' in environment file", signer_name);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
        }

        eprintln!("[Definition] No definition found");
        None
    }
}

#[derive(Debug)]
enum Reference {
    Input(String),    // input.name
    Flow(String),     // flow.name
    Variable(String), // var.name
    Action(String),   // action.name
    Signer(String),   // Reference to a signer by name
}

fn extract_reference_at_position(content: &str, position: &Position) -> Option<Reference> {
    let lines: Vec<&str> = content.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Check for variable definition pattern: variable "name" {
    let var_def_re = Regex::new(r#"variable\s+"([^"]+)""#).ok()?;
    for capture in var_def_re.captures_iter(line) {
        if let Some(name_match) = capture.get(1) {
            let name_start = name_match.start() as u32;
            let name_end = name_match.end() as u32;

            // If cursor is on the variable name in definition, treat it as a reference to itself
            if position.character >= name_start && position.character <= name_end {
                return Some(Reference::Variable(name_match.as_str().to_string()));
            }
        }
    }

    // Check for signer reference in signer = "name" format
    let signer_string_re = Regex::new(r#"signer\s*=\s*"([^"]+)""#).ok()?;
    for capture in signer_string_re.captures_iter(line) {
        if let Some(name_match) = capture.get(1) {
            // Get the position of just the name part (inside the quotes)
            let name_start = name_match.start() as u32;
            let name_end = name_match.end() as u32;

            // Check if cursor is within the name part specifically
            if position.character >= name_start && position.character <= name_end {
                return Some(Reference::Signer(name_match.as_str().to_string()));
            }
        }
    }

    // Check various reference patterns including signer.name
    let patterns: Vec<(&str, Box<dyn Fn(&str) -> Reference>)> = vec![
        (r"input\.(\w+)", Box::new(|name: &str| Reference::Input(name.to_string()))),
        (r"flow\.(\w+)", Box::new(|name: &str| Reference::Flow(name.to_string()))),
        (r"variable\.(\w+)", Box::new(|name: &str| Reference::Variable(name.to_string()))),  // Full form
        (r"var\.(\w+)", Box::new(|name: &str| Reference::Variable(name.to_string()))),       // Short form
        (r"action\.(\w+)", Box::new(|name: &str| Reference::Action(name.to_string()))),
        (r"signer\.(\w+)", Box::new(|name: &str| Reference::Signer(name.to_string()))),
    ];

    for (pattern, constructor) in patterns {
        let re = Regex::new(pattern).ok()?;

        for capture in re.captures_iter(line) {
            if let Some(name_match) = capture.get(1) {
                let full_match = capture.get(0)?;
                let start = full_match.start() as u32;
                let end = full_match.end() as u32;

                if position.character >= start && position.character <= end {
                    return Some(constructor(name_match.as_str()));
                }
            }
        }
    }

    None
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
}

fn find_signer_in_environment_files(uri: &Url, signer_name: &str) -> Option<Location> {
    // Get the directory of the current file
    let current_path = uri.to_file_path().ok()?;
    let dir = current_path.parent()?;

    // Extract environment from current file name if it exists
    let current_filename = current_path.file_name()?.to_str()?;
    let environment = if current_filename.ends_with(".tx") {
        let without_ext = &current_filename[..current_filename.len() - 3];
        let parts: Vec<&str> = without_ext.split('.').collect();
        if parts.len() >= 2 {
            Some(parts[parts.len() - 1])
        } else {
            None
        }
    } else {
        None
    };

    // Search in environment-specific files first, then global files
    let patterns = if let Some(env) = environment {
        vec![
            format!("*.{}.tx", env),     // Environment-specific files
            format!("signers.{}.tx", env), // Common signer file pattern
            "*.tx".to_string(),           // All .tx files as fallback
        ]
    } else {
        vec!["*.tx".to_string()]
    };

    // Search each pattern
    for pattern in patterns {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_str()?;
                        // Check if file matches pattern (simple glob matching)
                        if pattern == "*.tx" && name_str.ends_with(".tx") ||
                           pattern.ends_with(".tx") && name_str.ends_with(&pattern[1..]) {
                            // Read file and search for signer
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
            }
        }
    }

    None
}
