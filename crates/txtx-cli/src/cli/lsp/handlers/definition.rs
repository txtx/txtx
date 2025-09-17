//! Go-to-definition handler

use super::{Handler, TextDocumentHandler};
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;

pub struct DefinitionHandler {
    workspace: SharedWorkspaceState,
}

impl DefinitionHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let (uri, content, position) =
            self.get_document_at_position(&params.text_document_position_params)?;

        eprintln!("[Definition] Request at {}:{} in {}", position.line, position.character, uri);

        // Extract what's at the cursor position
        if let Some(var_ref) = extract_input_reference(&content, &position) {
            eprintln!("[Definition] Found input reference: {}", var_ref);
            let workspace = self.workspace.read();
            // Find the manifest for this runbook
            let manifest = workspace.get_manifest_for_runbook(&uri)?;

            // Look for variable definition in manifest
            if let Some(line) = find_variable_line(&manifest.uri, &var_ref) {
                eprintln!("[Definition] Found variable {} at line {} in manifest", var_ref, line);
                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: manifest.uri.clone(),
                    range: Range {
                        start: Position { line, character: 0 },
                        end: Position { line, character: 100 },
                    },
                }));
            } else {
                eprintln!("[Definition] Variable {} not found in manifest", var_ref);
            }
        }

        None
    }
}

impl Handler for DefinitionHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for DefinitionHandler {}

fn extract_input_reference(content: &str, position: &Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Look for input.variable_name or inputs.variable_name pattern
    let re = regex::Regex::new(r"inputs?\.(\w+)").ok()?;

    for capture in re.captures_iter(line) {
        if let Some(var_match) = capture.get(1) {
            let full_match = capture.get(0)?;
            let start = full_match.start() as u32;
            let end = full_match.end() as u32;

            if position.character >= start && position.character <= end {
                return Some(var_match.as_str().to_string());
            }
        }
    }

    None
}

fn find_variable_line(manifest_uri: &Url, var_name: &str) -> Option<u32> {
    // Look for the variable in the environments section of the manifest
    if let Ok(content) = std::fs::read_to_string(manifest_uri.path()) {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_environments = false;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check if we're entering environments section
            if trimmed.starts_with("environments:") {
                in_environments = true;
                continue;
            }

            // Check if we're leaving environments section (new top-level key)
            if in_environments
                && !line.starts_with(" ")
                && !line.starts_with("\t")
                && !trimmed.is_empty()
            {
                in_environments = false;
            }

            // Look for the variable within environments
            if in_environments && trimmed.starts_with(&format!("{}:", var_name)) {
                return Some(line_num as u32);
            }
        }
    }
    None
}
