//! LSP request handlers
//!
//! This module provides a trait-based system for handling LSP requests,
//! allowing each operation to be implemented in isolation.

use super::workspace::SharedWorkspaceState;
use lsp_types::*;

pub mod common;
mod completion;
mod debug_dump;
mod definition;
mod diagnostics;
mod document_sync;
mod environment_resolver;
mod hover;
pub mod references;
pub mod rename;
pub mod workspace;
mod workspace_discovery;

pub use common::is_manifest_file;
pub use completion::CompletionHandler;
pub use definition::DefinitionHandler;
pub use diagnostics::DiagnosticsHandler;
pub use document_sync::DocumentSyncHandler;
pub use hover::HoverHandler;
pub use references::ReferencesHandler;
pub use rename::RenameHandler;
pub use workspace::WorkspaceHandler;

/// Base trait for all LSP handlers
pub trait Handler: Send + Sync {
    /// Get the shared workspace state
    fn workspace(&self) -> &SharedWorkspaceState;
}

/// Trait for handlers that process text document requests
pub trait TextDocumentHandler: Handler {
    /// Get the URI and content for a text document position
    fn get_document_at_position(
        &self,
        params: &TextDocumentPositionParams,
    ) -> Option<(lsp_types::Url, String, Position)> {
        let workspace = self.workspace().read();
        let document = workspace.get_document(&params.text_document.uri)?;
        Some((params.text_document.uri.clone(), document.content().to_string(), params.position))
    }
}

/// Container for all handlers
#[derive(Clone)]
pub struct Handlers {
    pub completion: CompletionHandler,
    pub definition: DefinitionHandler,
    pub diagnostics: DiagnosticsHandler,
    pub hover: HoverHandler,
    pub document_sync: DocumentSyncHandler,
    pub references: ReferencesHandler,
    pub rename: RenameHandler,
    pub workspace: WorkspaceHandler,
}

impl Handlers {
    /// Create a new set of handlers sharing the same workspace
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        let workspace_handler = WorkspaceHandler::new(workspace.clone());
        Self {
            completion: CompletionHandler::new(workspace.clone()),
            definition: DefinitionHandler::new(workspace.clone()),
            diagnostics: DiagnosticsHandler::new(workspace.clone()),
            hover: HoverHandler::new(workspace.clone()),
            document_sync: DocumentSyncHandler::new(workspace.clone()),
            references: ReferencesHandler::new(workspace.clone()),
            rename: RenameHandler::new(workspace.clone()),
            workspace: workspace_handler,
        }
    }
}
