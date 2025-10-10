//! Common utilities shared across LSP handlers
//!
//! This module contains helper functions that are used by multiple handlers
//! to avoid duplication.

use crate::cli::lsp::workspace::{Manifest, SharedWorkspaceState};
use lsp_types::Url;

/// Filter URIs to include only files belonging to a specific runbook
///
/// # Arguments
/// * `uris` - List of URIs to filter
/// * `runbook_name` - Name of the runbook to filter by
/// * `workspace` - Shared workspace state for accessing manifest
///
/// # Returns
/// Vector of URIs that belong to the specified runbook, expanded to include
/// all files in multi-file runbooks
pub fn filter_runbook_uris(
    uris: &[Url],
    runbook_name: &str,
    workspace: &SharedWorkspaceState,
) -> Vec<Url> {
    let workspace_read = workspace.read();

    // Get manifest to map URIs to runbook names
    let manifest_uri = workspace_read
        .documents()
        .iter()
        .find(|(uri, _)| super::is_manifest_file(uri))
        .map(|(uri, _)| uri.clone());

    let Some(manifest_uri) = manifest_uri else {
        eprintln!("[Common] No manifest found for filtering runbooks");
        return Vec::new();
    };

    let Some(manifest) = workspace_read.get_manifest(&manifest_uri) else {
        eprintln!("[Common] Failed to get manifest");
        return Vec::new();
    };

    // Find the runbook with the matching name
    let matching_runbook = manifest
        .runbooks
        .iter()
        .find(|r| r.name == runbook_name);

    let Some(runbook) = matching_runbook else {
        eprintln!("[Common] Runbook '{}' not found in manifest", runbook_name);
        return Vec::new();
    };

    // Filter URIs to only include the matching runbook's URI
    let filtered_uris: Vec<Url> = uris
        .iter()
        .filter(|uri| {
            runbook
                .absolute_uri
                .as_ref()
                .map_or(false, |runbook_uri| runbook_uri == *uri)
        })
        .cloned()
        .collect();

    // Expand the filtered URIs
    expand_runbook_uris(&filtered_uris)
}

/// Expand runbook URIs to include all files in multi-file runbooks
///
/// For directory URIs (multi-file runbooks), this collects all .tx files
/// in the directory. For file URIs (single-file runbooks), returns them as-is.
///
/// # Arguments
/// * `uris` - List of runbook URIs (may be directories or files)
///
/// # Returns
/// Vector of file URIs with all .tx files from multi-file runbooks expanded
pub fn expand_runbook_uris(uris: &[Url]) -> Vec<Url> {
    let mut file_uris = Vec::new();

    for uri in uris {
        let Ok(path) = uri.to_file_path() else {
            eprintln!("[Common] Invalid file URI: {}", uri);
            continue;
        };

        if path.is_dir() {
            // Multi-file runbook: collect all .tx files
            let Ok(entries) = std::fs::read_dir(&path) else {
                eprintln!("[Common] Failed to read directory: {}", path.display());
                continue;
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.extension().map_or(false, |ext| ext == "tx") {
                    if let Ok(file_uri) = Url::from_file_path(&entry_path) {
                        file_uris.push(file_uri);
                    } else {
                        eprintln!("[Common] Failed to create URI for: {}", entry_path.display());
                    }
                }
            }
        } else {
            // Single file runbook
            file_uris.push(uri.clone());
        }
    }

    file_uris
}

/// Check if a URL points to a manifest file (txtx.yml)
pub fn is_manifest_file(uri: &Url) -> bool {
    uri.path().ends_with("txtx.yml") || uri.path().ends_with("txtx.yaml")
}
