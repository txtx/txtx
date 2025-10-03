//! Code completion handler

use super::{Handler, TextDocumentHandler};
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;
use std::collections::HashSet;

#[derive(Clone)]
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

        if !is_after_input_dot(&content, &position) {
            return None;
        }

        let workspace = self.workspace.read();
        let manifest = workspace.get_manifest_for_runbook(&uri)?;

        // Collect unique input names from all environments, deduplicating
        // to avoid showing the same completion multiple times
        let unique_inputs: HashSet<_> = manifest
            .environments
            .values()
            .flat_map(|vars| vars.keys())
            .collect();

        // Transform to completion items
        let items: Vec<CompletionItem> = unique_inputs
            .into_iter()
            .map(|input| CompletionItem {
                label: input.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                ..Default::default()
            })
            .collect();

        Some(CompletionResponse::Array(items))
    }
}

impl Handler for CompletionHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for CompletionHandler {}

fn is_after_input_dot(content: &str, position: &Position) -> bool {
    const INPUT_DOT: &str = "input.";
    const INPUT_DOT_LEN: usize = INPUT_DOT.len();

    content
        .lines()
        .nth(position.line as usize)
        .and_then(|line| {
            let end = position.character as usize;
            let start = end.saturating_sub(INPUT_DOT_LEN);
            line.get(start..end)
        })
        .is_some_and(|slice| slice == INPUT_DOT)
}
