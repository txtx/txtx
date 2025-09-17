//! Code completion handler

use super::{Handler, TextDocumentHandler};
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;
use std::collections::HashSet;

pub struct CompletionHandler {
    workspace: SharedWorkspaceState,
}

impl CompletionHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }

    pub fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let (uri, content, position) =
            self.get_document_at_position(&params.text_document_position)?;

        // Check if we're after "input."
        if is_after_input_dot(&content, &position) {
            let workspace = self.workspace.read();
            let manifest = workspace.get_manifest_for_runbook(&uri)?;

            // Collect all available inputs from environments
            let mut inputs = HashSet::new();
            for vars in manifest.environments.values() {
                for key in vars.keys() {
                    inputs.insert(key.clone());
                }
            }

            // Create completion items
            let items: Vec<CompletionItem> = inputs
                .into_iter()
                .map(|input| CompletionItem {
                    label: input,
                    kind: Some(CompletionItemKind::VARIABLE),
                    ..Default::default()
                })
                .collect();

            return Some(CompletionResponse::Array(items));
        }

        None
    }
}

impl Handler for CompletionHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for CompletionHandler {}

fn is_after_input_dot(content: &str, position: &Position) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    if let Some(line) = lines.get(position.line as usize) {
        if position.character >= 6 {
            let start = (position.character - 6) as usize;
            let end = position.character as usize;
            if let Some(slice) = line.get(start..end) {
                return slice == "input.";
            }
        }
    }
    false
}
