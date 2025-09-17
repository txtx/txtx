//! File system scanning utilities for LSP
//!
//! Provides functionality for finding files and workspace roots

use std::path::{Path, PathBuf};
use std::fs;

/// Find the root directory containing txtx.yml
pub fn find_txtx_yml_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() { 
        start_path.parent()? 
    } else { 
        start_path 
    };

    loop {
        for name in &["txtx.yml", "txtx.yaml"] {
            if current.join(name).exists() {
                return Some(current.to_path_buf());
            }
        }

        current = current.parent()?;
    }
}

/// Find all .tx files in a directory
pub fn find_tx_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut tx_files = Vec::new();
    find_tx_files_recursive(dir, &mut tx_files, 0)?;
    Ok(tx_files)
}

fn find_tx_files_recursive(dir: &Path, tx_files: &mut Vec<PathBuf>, depth: usize) -> std::io::Result<()> {
    // Limit depth to prevent infinite recursion
    if depth > 5 {
        return Ok(());
    }

    // Skip common directories we don't want to scan
    if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
        if matches!(dir_name, "node_modules" | ".git" | "target" | ".vscode" | ".idea") {
            return Ok(());
        }
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            find_tx_files_recursive(&path, tx_files, depth + 1)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("tx") {
            tx_files.push(path);
        }
    }

    Ok(())
}

