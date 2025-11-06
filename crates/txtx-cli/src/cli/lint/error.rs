//! Error types for the linter module

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during linting operations
#[derive(Debug, Error)]
pub enum LinterError {
    /// Failed to load or parse a manifest file
    #[error("Failed to load manifest from {path}: {message}")]
    ManifestLoad {
        /// Path to the manifest file
        path: PathBuf,
        /// Error message
        message: String,
    },

    /// Requested runbook was not found in the workspace
    #[error("Runbook '{0}' not found")]
    RunbookNotFound(String),

    /// Failed to resolve runbook sources from manifest
    #[error("Failed to resolve runbook sources: {0}")]
    RunbookResolution(String),

    /// IO error occurred during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration file already exists
    #[error("Configuration file {0} already exists")]
    ConfigExists(PathBuf),

    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Generic error for miscellaneous issues
    #[error("{0}")]
    Other(String),
}
