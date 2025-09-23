//! Document synchronization handler
//!
//! Handles document lifecycle events: open, change, save, close

use super::Handler;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;

pub struct DocumentSyncHandler {
    workspace: SharedWorkspaceState,
}

impl DocumentSyncHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }

    /// Handle document open
    pub fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;

        self.workspace.write().open_document(uri, content);
    }

    /// Handle document change
    pub fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // For now, we only support full document sync
        if let Some(change) = params.content_changes.into_iter().next() {
            self.workspace.write().update_document(&uri, change.text);
        }
    }

    /// Handle document save
    #[allow(dead_code)]
    pub fn did_save(&self, params: DidSaveTextDocumentParams) -> Option<PublishDiagnosticsParams> {
        let uri = &params.text_document.uri;

        // Trigger validation on save
        let workspace = self.workspace.read();
        let document = workspace.get_document(uri)?;

        let diagnostics = if document.is_runbook() {
            // Try to get manifest for enhanced validation
            let manifest = workspace.get_manifest_for_document(uri);

            if let Some(manifest) = manifest {
                crate::cli::lsp::diagnostics_multi_file::validate_with_multi_file_support(
                    uri,
                    document.content(),
                    Some(manifest),
                    None, // TODO: Get environment from workspace
                    &[],  // TODO: Get CLI inputs from workspace
                )
            } else {
                // Fall back to basic validation
                crate::cli::lsp::diagnostics::validate_runbook(uri, document.content())
            }
        } else {
            Vec::new()
        };

        Some(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: Some(document.version()),
        })
    }

    /// Handle document close
    pub fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.workspace.write().close_document(&uri);
    }
}

impl Handler for DocumentSyncHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_lifecycle() {
        let workspace = SharedWorkspaceState::new();
        let handler = DocumentSyncHandler::new(workspace.clone());

        let uri = Url::parse("file:///test.tx").unwrap();

        // Open document
        handler.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "txtx".to_string(),
                version: 1,
                text: "initial content".to_string(),
            },
        });

        // Verify document was opened
        {
            let ws = workspace.read();
            assert!(ws.get_document(&uri).is_some());
        }

        // Change document
        handler.did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri: uri.clone(), version: 2 },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "updated content".to_string(),
            }],
        });

        // Verify content was updated
        {
            let ws = workspace.read();
            let doc = ws.get_document(&uri).unwrap();
            assert_eq!(doc.content(), "updated content");
        }

        // Close document
        handler.did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        });

        // Verify document was closed
        {
            let ws = workspace.read();
            assert!(ws.get_document(&uri).is_none());
        }
    }
}
