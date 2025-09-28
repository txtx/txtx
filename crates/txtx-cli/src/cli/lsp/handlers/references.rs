//! Find references handler for txtx LSP
//!
//! Finds all references to a symbol across all environment files, not just the current environment.

use super::workspace_discovery::{discover_workspace_files, find_input_in_yaml};
use super::{Handler, TextDocumentHandler};
use crate::cli::lsp::hcl_ast::{self, Reference};
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::{Location, Position, Range, ReferenceParams, Url};
use regex::Regex;
use std::collections::HashSet;

#[derive(Clone)]
pub struct ReferencesHandler {
    workspace: SharedWorkspaceState,
}

impl ReferencesHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }

    /// Determine which runbook a file belongs to, if any.
    ///
    /// Returns None if:
    /// - No manifest is found
    /// - File is not part of any runbook (workspace-wide file)
    fn get_runbook_for_file(&self, file_uri: &Url) -> Option<String> {
        let workspace_read = self.workspace.read();

        // Get manifest
        let manifest_uri = workspace_read
            .documents()
            .iter()
            .find(|(uri, _)| super::is_manifest_file(uri))
            .map(|(uri, _)| uri.clone())?;

        let manifest = workspace_read.get_manifest(&manifest_uri)?;

        // Use existing helper from multi_file module
        crate::cli::lsp::multi_file::get_runbook_name_for_file(file_uri, manifest)
    }

    /// Find all references to the symbol at the given position
    pub fn find_references(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get the content and find what symbol we're looking for
        let workspace = self.workspace.read();
        let document = workspace.get_document(uri)?;
        let content = document.content();

        // Extract the reference at cursor position
        let reference = extract_reference_at_position(content, position)?;

        eprintln!("[References] Looking for references to: {:?}", reference);

        // Determine current runbook for scoping
        let current_runbook = self.get_runbook_for_file(uri);

        let mut locations = Vec::new();
        let mut searched_uris = HashSet::new();

        // Get manifest for runbook filtering
        let manifest_uri = workspace
            .documents()
            .iter()
            .find(|(uri, _)| super::is_manifest_file(uri))
            .map(|(uri, _)| uri.clone());

        let manifest = manifest_uri.as_ref().and_then(|uri| workspace.get_manifest(uri));

        // Search open documents with optional runbook filtering
        let is_scoped = !reference.is_workspace_scoped() && current_runbook.is_some();

        for (doc_uri, doc) in workspace.documents() {
            // Filter by runbook if this is a runbook-scoped reference
            if is_scoped {
                let doc_runbook = manifest.as_ref().and_then(|m| {
                    crate::cli::lsp::multi_file::get_runbook_name_for_file(doc_uri, m)
                });

                // Skip if document is in a different runbook
                if doc_runbook.as_ref() != current_runbook.as_ref() {
                    continue;
                }
            }

            let doc_content = doc.content();

            // Find all occurrences in this document
            let occurrences = find_all_occurrences(doc_content, &reference);

            for occurrence in occurrences {
                locations.push(Location {
                    uri: doc_uri.clone(),
                    range: occurrence,
                });
            }

            searched_uris.insert(doc_uri.clone());
        }

        // Release the read lock before discovering files
        drop(workspace);

        // Discover workspace files (manifest + all runbooks)
        let discovered = discover_workspace_files(&self.workspace);

        // Search manifest for Input references in YAML
        if let Some(manifest_uri) = &discovered.manifest_uri {
            if let Reference::Input(input_name) = &reference {
                self.search_manifest_for_input(
                    manifest_uri,
                    input_name,
                    &mut locations,
                    &mut searched_uris,
                );
            }
        }

        // Search runbooks from manifest (even if not open)
        // Expand directory URIs into individual .tx files for multi-file runbooks
        // Filter by runbook if the reference type is runbook-scoped
        let file_uris = match (reference.is_workspace_scoped(), self.get_runbook_for_file(uri)) {
            // Workspace-scoped: search all runbooks
            (true, _) => expand_runbook_uris(&discovered.runbook_uris),
            // Runbook-scoped with known runbook: filter to that runbook only
            (false, Some(runbook_name)) => {
                filter_runbook_uris(&discovered.runbook_uris, &runbook_name, &self.workspace)
            }
            // Runbook-scoped but no runbook found: treat as workspace-wide (loose files)
            (false, None) => expand_runbook_uris(&discovered.runbook_uris),
        };

        for file_uri in &file_uris {
            self.search_runbook_for_references(
                file_uri,
                &reference,
                &mut locations,
                &searched_uris,
            );
        }

        eprintln!("[References] Found {} references across {} files",
                  locations.len(),
                  locations.iter().map(|l| &l.uri).collect::<HashSet<_>>().len());

        Some(locations)
    }

    /// Search manifest file for input references in YAML
    ///
    /// Note: Always searches manifest even if already in searched_uris, because
    /// we need YAML-specific pattern matching (not just .tx file patterns)
    fn search_manifest_for_input(
        &self,
        manifest_uri: &Url,
        input_name: &str,
        locations: &mut Vec<Location>,
        _searched_uris: &mut HashSet<Url>,
    ) {
        // Read manifest from disk
        let content = manifest_uri
            .to_file_path()
            .ok()
            .and_then(|path| std::fs::read_to_string(&path).ok());

        if let Some(content) = content {
            let yaml_occurrences = find_input_in_yaml(&content, input_name);
            locations.extend(yaml_occurrences.into_iter().map(|range| Location {
                uri: manifest_uri.clone(),
                range,
            }));
        }
    }

    /// Search a runbook file for references (reads from disk if not already open)
    fn search_runbook_for_references(
        &self,
        runbook_uri: &Url,
        reference: &Reference,
        locations: &mut Vec<Location>,
        searched_uris: &HashSet<Url>,
    ) {
        // Skip if already searched as open document
        if searched_uris.contains(runbook_uri) {
            return;
        }

        // Read from disk and search
        if let Some(runbook_content) = runbook_uri
            .to_file_path()
            .ok()
            .and_then(|path| std::fs::read_to_string(&path).ok())
        {
            let occurrences = find_all_occurrences(&runbook_content, reference);
            locations.extend(occurrences.into_iter().map(|range| Location {
                uri: runbook_uri.clone(),
                range,
            }));
        }
    }
}

impl Handler for ReferencesHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for ReferencesHandler {}

/// Extract what symbol is being referenced at the given position
pub fn extract_reference_at_position(content: &str, position: Position) -> Option<Reference> {
    eprintln!("[extract_reference] Position: {}:{}", position.line, position.character);

    let line = content.lines().nth(position.line as usize)?;
    let char_idx = position.character as usize;

    eprintln!("[extract_reference] Line: '{}'", line);
    eprintln!("[extract_reference] Char idx: {}", char_idx);

    // First check if we're in a YAML manifest file (inputs section)
    if let Some(input_ref) = extract_yaml_input_at_position(content, position, line, char_idx) {
        eprintln!("[extract_reference] Found YAML input: {:?}", input_ref);
        return Some(input_ref);
    }

    eprintln!("[extract_reference] No YAML input found, trying AST-based extraction");

    // Use AST-based extraction for .tx files
    let (reference, _range) = hcl_ast::extract_reference_at_position(content, position)?;

    // Filter out Output references (not supported)
    match reference {
        Reference::Output(_) => {
            eprintln!("[extract_reference] Ignoring Output reference (not supported)");
            None
        }
        _ => Some(reference),
    }
}

/// Extract input reference from YAML manifest file when clicking on a key
///
/// In txtx manifests, inputs are defined directly under environment:
/// ```yaml
/// environments:
///   global:
///     chain_id: 11155111  <- clicking here should detect "chain_id" as an Input
/// ```
fn extract_yaml_input_at_position(
    content: &str,
    position: Position,
    line: &str,
    char_idx: usize,
) -> Option<Reference> {
    // Match YAML key pattern: optional whitespace + key_name + colon
    let re = Regex::new(r"^\s*(\w+):\s*").ok()?;
    let cap = re.captures(line)?;
    let name_match = cap.get(1)?;
    let key_name = name_match.as_str();

    // Check if cursor is on the key name
    if char_idx < name_match.start() || char_idx > name_match.end() {
        return None;
    }

    // Parse YAML and check if this key exists under any environment
    if is_key_under_environments(content, key_name) {
        return Some(Reference::Input(key_name.to_string()));
    }

    None
}

/// Check if a key exists under any environment in the YAML structure
///
/// Structure: environments -> [env_name] -> [key: value]
fn is_key_under_environments(content: &str, key_name: &str) -> bool {
    // Parse YAML structure
    let Ok(yaml_value) = serde_yml::from_str::<serde_yml::Value>(content) else {
        return false;
    };

    let Some(yaml_mapping) = yaml_value.as_mapping() else {
        return false;
    };

    // Get environments section
    let Some(envs_section) = yaml_mapping.get(&serde_yml::Value::String("environments".to_string())) else {
        return false;
    };

    let Some(envs_mapping) = envs_section.as_mapping() else {
        return false;
    };

    // Iterate through each environment (global, sepolia, etc.)
    for (env_key, env_value) in envs_mapping {
        let Some(env_mapping) = env_value.as_mapping() else {
            continue;
        };

        // Check if this key exists under this environment
        if env_mapping.contains_key(&serde_yml::Value::String(key_name.to_string())) {
            return true;
        }
    }

    false
}

/// Expand runbook URIs into individual file URIs
///
/// For multi-file runbooks (directories), this scans the directory and returns URIs
/// for all .tx files. For single-file runbooks, returns the URI as-is.
fn expand_runbook_uris(uris: &[Url]) -> Vec<Url> {
    let mut file_uris = Vec::new();

    for uri in uris {
        let Ok(path) = uri.to_file_path() else {
            eprintln!("[References] Invalid file URI: {}", uri);
            continue;
        };

        if path.is_dir() {
            // Multi-file runbook: collect all .tx files
            let Ok(entries) = std::fs::read_dir(&path) else {
                eprintln!("[References] Failed to read directory: {}", path.display());
                continue;
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.extension().map_or(false, |ext| ext == "tx") {
                    if let Ok(file_uri) = Url::from_file_path(&entry_path) {
                        file_uris.push(file_uri);
                    } else {
                        eprintln!("[References] Failed to create URI for: {}", entry_path.display());
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

/// Filter runbook URIs to only include files from a specific runbook
///
/// This uses the manifest to determine which URIs belong to the specified runbook,
/// then expands those URIs into individual .tx files.
fn filter_runbook_uris(
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
        eprintln!("[References] No manifest found for filtering runbooks");
        return Vec::new();
    };

    let Some(manifest) = workspace_read.get_manifest(&manifest_uri) else {
        eprintln!("[References] Failed to get manifest");
        return Vec::new();
    };

    // Find the runbook with the matching name
    let matching_runbook = manifest
        .runbooks
        .iter()
        .find(|r| r.name == runbook_name);

    let Some(runbook) = matching_runbook else {
        eprintln!("[References] Runbook '{}' not found in manifest", runbook_name);
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

/// Find all occurrences of a reference in the given content
pub fn find_all_occurrences(content: &str, reference: &Reference) -> Vec<Range> {
    // Use AST-based occurrence finding directly
    hcl_ast::find_all_occurrences(content, reference)
}
