//! Rename handler for txtx LSP
//!
//! Renames symbols across ALL environment files to maintain consistency.

use super::common::{expand_runbook_uris, filter_runbook_uris};
use super::references::{extract_reference_at_position, find_all_occurrences};
use super::workspace_discovery::{discover_workspace_files, find_input_in_yaml};
use super::{Handler, TextDocumentHandler};
use crate::cli::lsp::hcl_ast::Reference;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::{PrepareRenameResponse, RenameParams, TextDocumentPositionParams, TextEdit, Url, WorkspaceEdit};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct RenameHandler {
    workspace: SharedWorkspaceState,
}

impl RenameHandler {
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

    /// Prepare for rename - check if the symbol at the position can be renamed
    pub fn prepare_rename(&self, params: TextDocumentPositionParams) -> Option<PrepareRenameResponse> {
        let uri = &params.text_document.uri;
        let position = params.position;

        eprintln!("[PrepareRename] Getting workspace...");

        // Get the content and find what symbol we're checking
        let workspace = self.workspace.read();

        eprintln!("[PrepareRename] Getting document for URI: {:?}", uri);
        let document = workspace.get_document(uri);

        if document.is_none() {
            eprintln!("[PrepareRename] ERROR: Document not found!");
            return None;
        }

        let document = document?;
        let content = document.content();

        eprintln!("[PrepareRename] Content length: {}", content.len());

        // Extract the reference at cursor position
        eprintln!("[PrepareRename] Extracting reference...");
        let reference = extract_reference_at_position(content, position)?;

        eprintln!("[PrepareRename] Found reference: {:?}", reference);

        // Find the range of the symbol at the cursor position
        // For YAML files, also check YAML patterns
        eprintln!("[PrepareRename] Searching for occurrences...");
        let mut occurrences = find_all_occurrences(content, &reference);
        eprintln!("[PrepareRename] find_all_occurrences returned {} items", occurrences.len());

        // If this is a YAML file and we're looking for an Input, also search YAML patterns
        if let Reference::Input(input_name) = &reference {
            eprintln!("[PrepareRename] Checking if YAML file...");
            if uri.path().ends_with(".yml") || uri.path().ends_with(".yaml") {
                eprintln!("[PrepareRename] Is YAML file, searching for YAML patterns...");
                let yaml_occurrences = find_input_in_yaml(content, input_name);
                eprintln!("[PrepareRename] find_input_in_yaml returned {} items", yaml_occurrences.len());
                occurrences.extend(yaml_occurrences);
            }
        }

        eprintln!("[PrepareRename] Total occurrences: {}", occurrences.len());

        let range = occurrences.iter().find(|r| {
            r.start.line <= position.line
                && position.line <= r.end.line
                && r.start.character <= position.character
                && position.character <= r.end.character
        })?;

        eprintln!("[PrepareRename] Found range: {:?}", range);

        // Return the range and placeholder (current name)
        Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: *range,
            placeholder: reference.name().to_string(),
        })
    }

    /// Rename the symbol at the given position across all files
    pub fn rename(&self, params: RenameParams) -> Option<WorkspaceEdit> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        eprintln!("[Rename Handler] URI: {:?}", uri);
        eprintln!("[Rename Handler] Position: {}:{}", position.line, position.character);

        // Get the content and find what symbol we're renaming
        let workspace = self.workspace.read();
        let document = workspace.get_document(uri);

        if document.is_none() {
            eprintln!("[Rename Handler] ERROR: Document not found in workspace!");
            eprintln!("[Rename Handler] Available documents:");
            for (doc_uri, _) in workspace.documents() {
                eprintln!("[Rename Handler]   - {:?}", doc_uri);
            }
            return None;
        }

        let document = document?;
        let content = document.content();
        eprintln!("[Rename Handler] Document content length: {}", content.len());

        // Extract the reference at cursor position
        let reference = extract_reference_at_position(content, position)?;

        eprintln!("[Rename] Renaming {:?} to '{}'", reference, new_name);

        // Determine current runbook for scoping
        let current_runbook = self.get_runbook_for_file(uri);

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
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

            if !occurrences.is_empty() {
                let edits: Vec<TextEdit> = occurrences
                    .into_iter()
                    .map(|range| TextEdit {
                        range,
                        new_text: new_name.clone(),
                    })
                    .collect();

                changes.insert(doc_uri.clone(), edits);
            }

            searched_uris.insert(doc_uri.clone());
        }

        // Release the read lock before discovering files
        drop(workspace);

        // Discover workspace files (manifest + all runbooks)
        eprintln!("[Rename] Discovering workspace files...");
        let discovered = discover_workspace_files(&self.workspace);
        eprintln!("[Rename] Found manifest: {:?}", discovered.manifest_uri);
        eprintln!("[Rename] Found {} runbooks", discovered.runbook_uris.len());

        // Search manifest for Input references in YAML
        if let Some(manifest_uri) = &discovered.manifest_uri {
            if let Reference::Input(input_name) = &reference {
                eprintln!("[Rename] Searching manifest for Input: {}", input_name);
                self.rename_in_manifest(
                    manifest_uri,
                    input_name,
                    new_name,
                    &mut changes,
                    &mut searched_uris,
                );
            }
        }

        // Search runbooks from manifest (even if not open)
        // Filter by runbook if the reference type is runbook-scoped
        let file_uris = match (reference.is_workspace_scoped(), current_runbook) {
            // Workspace-scoped: search all runbooks
            (true, _) => expand_runbook_uris(&discovered.runbook_uris),
            // Runbook-scoped with known runbook: filter to that runbook only
            (false, Some(runbook_name)) => {
                filter_runbook_uris(&discovered.runbook_uris, &runbook_name, &self.workspace)
            }
            // Runbook-scoped but no runbook found: treat as workspace-wide (loose files)
            (false, None) => expand_runbook_uris(&discovered.runbook_uris),
        };

        eprintln!("[Rename] Searching {} files...", file_uris.len());
        for file_uri in &file_uris {
            eprintln!("[Rename] Checking file: {:?}", file_uri);
            self.rename_in_runbook(
                file_uri,
                &reference,
                new_name,
                &mut changes,
                &searched_uris,
            );
        }

        eprintln!("[Rename] Generated edits for {} files", changes.len());

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Generate rename edits for input references in manifest YAML
    ///
    /// Note: Always searches manifest even if already in searched_uris, because
    /// we need YAML-specific pattern matching (not just .tx file patterns)
    fn rename_in_manifest(
        &self,
        manifest_uri: &Url,
        input_name: &str,
        new_name: &str,
        changes: &mut HashMap<Url, Vec<TextEdit>>,
        _searched_uris: &mut HashSet<Url>,
    ) {
        // Read manifest from disk and generate edits
        let content = manifest_uri
            .to_file_path()
            .ok()
            .and_then(|path| std::fs::read_to_string(&path).ok());

        if let Some(content) = content {
            let yaml_occurrences = find_input_in_yaml(&content, input_name);

            if !yaml_occurrences.is_empty() {
                let edits: Vec<TextEdit> = yaml_occurrences
                    .into_iter()
                    .map(|range| TextEdit {
                        range,
                        new_text: new_name.to_string(),
                    })
                    .collect();

                changes
                    .entry(manifest_uri.clone())
                    .or_insert_with(Vec::new)
                    .extend(edits);
            }
        }
    }

    /// Generate rename edits for a runbook file or directory (reads from disk if not already open)
    fn rename_in_runbook(
        &self,
        runbook_uri: &Url,
        reference: &Reference,
        new_name: &str,
        changes: &mut HashMap<Url, Vec<TextEdit>>,
        searched_uris: &HashSet<Url>,
    ) {
        // Skip if already searched as open document
        if searched_uris.contains(runbook_uri) {
            eprintln!("[rename_in_runbook] Skipping (already searched): {:?}", runbook_uri);
            return;
        }

        // Check if this is a file or directory
        let path = match runbook_uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("[rename_in_runbook] Invalid file path: {:?}", runbook_uri);
                return;
            }
        };

        if path.is_file() {
            // Single file runbook
            eprintln!("[rename_in_runbook] Processing single file: {:?}", path);
            self.rename_in_file(runbook_uri, reference, new_name, changes);
        } else if path.is_dir() {
            // Multi-file runbook - search all .tx files in directory
            eprintln!("[rename_in_runbook] Processing directory: {:?}", path);

            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() && entry_path.extension().map_or(false, |ext| ext == "tx") {
                        eprintln!("[rename_in_runbook] Found .tx file: {:?}", entry_path);

                        if let Ok(file_uri) = Url::from_file_path(&entry_path) {
                            // Skip if already searched
                            if !searched_uris.contains(&file_uri) {
                                self.rename_in_file(&file_uri, reference, new_name, changes);
                            }
                        }
                    }
                }
            }
        } else {
            eprintln!("[rename_in_runbook] Path doesn't exist: {:?}", path);
        }
    }

    /// Helper to rename in a single file
    fn rename_in_file(
        &self,
        file_uri: &Url,
        reference: &Reference,
        new_name: &str,
        changes: &mut HashMap<Url, Vec<TextEdit>>,
    ) {
        eprintln!("[rename_in_file] Reading: {:?}", file_uri);

        if let Some(content) = file_uri
            .to_file_path()
            .ok()
            .and_then(|path| std::fs::read_to_string(&path).ok())
        {
            eprintln!("[rename_in_file] Successfully read {} bytes", content.len());
            let occurrences = find_all_occurrences(&content, reference);
            eprintln!("[rename_in_file] Found {} occurrences", occurrences.len());

            if !occurrences.is_empty() {
                let num_occurrences = occurrences.len();
                let edits: Vec<TextEdit> = occurrences
                    .into_iter()
                    .map(|range| TextEdit {
                        range,
                        new_text: new_name.to_string(),
                    })
                    .collect();
                changes.insert(file_uri.clone(), edits);
                eprintln!("[rename_in_file] Added {} edits for {:?}", num_occurrences, file_uri);
            }
        } else {
            eprintln!("[rename_in_file] Failed to read file: {:?}", file_uri);
        }
    }
}

impl Handler for RenameHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for RenameHandler {}
