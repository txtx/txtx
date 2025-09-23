use std::path::{Path, PathBuf};
use txtx_core::manifest::WorkspaceManifest;

/// Handles workspace and runbook discovery
pub struct WorkspaceAnalyzer {
    pub manifest_path: PathBuf,
}

impl WorkspaceAnalyzer {
    pub fn new(manifest_path: PathBuf) -> Self {
        Self { manifest_path }
    }

    /// Find a specific runbook in the manifest
    pub fn find_runbook_in_manifest(
        &self,
        manifest: &WorkspaceManifest,
        runbook_name: &str,
    ) -> Option<RunbookLocation> {
        for metadata in &manifest.runbooks {
            if metadata.name == runbook_name {
                let base_path = self.manifest_path.parent().unwrap_or(Path::new("."));
                let runbook_path = base_path.join(&metadata.location);

                return Some(RunbookLocation { name: metadata.name.clone(), path: runbook_path });
            }
        }
        None
    }

    /// Find all runbooks in the manifest
    pub fn find_all_runbooks_in_manifest(
        &self,
        manifest: &WorkspaceManifest,
    ) -> Vec<(String, RunbookLocation)> {
        let mut runbooks = Vec::new();
        let base_path = self.manifest_path.parent().unwrap_or(Path::new("."));

        for metadata in &manifest.runbooks {
            let runbook_path = base_path.join(&metadata.location);
            let location = RunbookLocation { name: metadata.name.clone(), path: runbook_path };
            runbooks.push((metadata.name.clone(), location));
        }

        runbooks
    }

    /// Find runbooks in the current directory when no manifest exists
    pub fn find_runbooks_in_directory() -> Result<Vec<PathBuf>, String> {
        let current_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;

        let possible_files = vec!["main.tx", "txtx.tx", "runbook.tx"];

        // First try common filenames
        for file in &possible_files {
            let path = current_dir.join(file);
            if path.exists() {
                return Ok(vec![path]);
            }
        }

        // Then look for any .tx files
        let entries = std::fs::read_dir(&current_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut tx_files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "tx") {
                    tx_files.push(path);
                }
            }
        }

        if tx_files.is_empty() {
            Err("No runbook files found in current directory".to_string())
        } else {
            Ok(tx_files)
        }
    }
}

/// Represents a runbook location
pub struct RunbookLocation {
    pub name: String,
    pub path: PathBuf,
}

impl RunbookLocation {
    /// Get the actual file path for the runbook
    #[allow(dead_code)]
    pub fn file_path(&self) -> PathBuf {
        if self.path.is_dir() {
            // If it's a directory, look for main.tx
            self.path.join("main.tx")
        } else if self.path.exists() {
            self.path.clone()
        } else {
            // Try with .tx extension
            self.path.with_extension("tx")
        }
    }

    /// Check if the runbook file exists
    #[allow(dead_code)]
    pub fn exists(&self) -> bool {
        self.file_path().exists()
    }
}
