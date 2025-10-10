//! Workspace state management for LSP
//!
//! # C4 Architecture Annotations
//! @c4-component WorkspaceState
//! @c4-container LSP Server
//! @c4-description Manages open documents, manifests, and validation state
//! @c4-technology Rust (DashMap for concurrent access)
//! @c4-responsibility Track open documents and their content
//! @c4-responsibility Maintain manifest-to-runbook relationships
//! @c4-responsibility Coordinate validation state across workspace

mod dependency_extractor;
mod dependency_graph;
mod documents;
pub mod manifest_converter;
mod manifests;
mod state;
mod validation_state;

pub use documents::Document;
pub use manifests::Manifest;
#[cfg(test)]
pub use manifests::RunbookRef;
pub use state::{SharedWorkspaceState, WorkspaceState};
pub use validation_state::ValidationStatus;
