//! Multi-file runbook support for LSP
//!
//! This module provides functionality to handle multi-file runbooks in the LSP,
//! similar to how the doctor command processes them.

use lsp_types::Url;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_core::manifest::file::read_runbook_from_location;

/// Information about a multi-file runbook
#[derive(Debug, Clone)]
pub struct MultiFileRunbook {
    /// The root directory of the runbook
    pub root_dir: PathBuf,
    /// Map of file URIs to their content
    pub files: HashMap<Url, String>,
    /// Combined content for validation
    pub combined_content: String,
    /// File boundaries for error mapping: (file_path, start_line, end_line)
    pub file_boundaries: Vec<(String, usize, usize)>,
}

/// Check if a file is part of a multi-file runbook
pub fn is_multi_file_runbook(file_uri: &Url) -> Option<PathBuf> {
    let file_path = PathBuf::from(file_uri.path());

    // Check if the parent directory is a runbook directory
    if let Some(parent) = file_path.parent() {
        // Look for main.tx in the parent directory
        let main_file = parent.join("main.tx");
        if main_file.exists() && main_file != file_path {
            return Some(parent.to_path_buf());
        }
    }

    None
}

/// Load all files from a multi-file runbook
pub fn load_multi_file_runbook(
    root_dir: &Path,
    runbook_name: &str,
    environment: Option<&str>,
) -> Result<MultiFileRunbook, String> {
    let file_location = FileLocation::from_path_string(&root_dir.to_string_lossy())?;

    // Use the same function as doctor to load the runbook
    let (_, _, runbook_sources) = read_runbook_from_location(
        &file_location,
        &Some(runbook_name.to_string()),
        &environment.map(|e| e.to_string()),
        Some(runbook_name),
    )?;

    let mut files = HashMap::new();
    let mut combined_content = String::new();
    let mut file_boundaries = Vec::new();
    let mut current_line = 1usize;

    // Process each file in the runbook
    for (file_location, (_name, raw_content)) in &runbook_sources.tree {
        let file_path = PathBuf::from(file_location.to_string());
        let file_uri = Url::from_file_path(&file_path)
            .map_err(|_| format!("Invalid file path: {}", file_path.display()))?;

        let start_line = current_line;
        let content = raw_content.to_string();

        // Add to combined content
        combined_content.push_str(&content);
        combined_content.push('\n');

        // Track boundaries
        let line_count = content.lines().count();
        current_line += line_count + 1;
        file_boundaries.push((file_location.to_string(), start_line, current_line));

        // Store individual file content
        files.insert(file_uri, content);
    }

    Ok(MultiFileRunbook {
        root_dir: root_dir.to_path_buf(),
        files,
        combined_content,
        file_boundaries,
    })
}

/// Map a line number from combined content back to the original file
pub fn map_line_to_file(
    line: usize,
    file_boundaries: &[(String, usize, usize)],
) -> Option<(String, usize)> {
    for (file_path, start_line, end_line) in file_boundaries {
        if line >= *start_line && line < *end_line {
            let mapped_line = line - start_line + 1;
            return Some((file_path.clone(), mapped_line));
        }
    }
    None
}

/// Get the runbook name from a manifest for a given file
pub fn get_runbook_name_for_file(
    file_uri: &Url,
    manifest: &crate::cli::lsp::workspace::Manifest,
) -> Option<String> {
    let file_path = PathBuf::from(file_uri.path());
    eprintln!("[DEBUG] get_runbook_name_for_file: checking file_path: {:?}", file_path);
    eprintln!("[DEBUG] Manifest has {} runbooks", manifest.runbooks.len());

    // Check each runbook in the manifest
    for runbook in &manifest.runbooks {
        eprintln!("[DEBUG] Checking runbook: {} with location: {}", runbook.name, runbook.location);
        let runbook_path = if let Some(base) = manifest.uri.to_file_path().ok() {
            base.parent()?.join(&runbook.location)
        } else {
            PathBuf::from(&runbook.location)
        };

        eprintln!("[DEBUG] Checking if {:?} starts with {:?}", file_path, runbook_path);
        // Check if the file is inside this runbook's directory
        if file_path.starts_with(&runbook_path) {
            eprintln!("[DEBUG] Match found! Returning runbook name: {}", runbook.name);
            return Some(runbook.name.clone());
        }
    }

    None
}
