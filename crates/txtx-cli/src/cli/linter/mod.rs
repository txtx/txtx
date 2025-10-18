//! Linter for txtx runbooks
//!
//! # C4 Architecture Annotations
//! @c4-component Linter Engine
//! @c4-container txtx-cli
//! @c4-description Orchestrates validation pipeline for runbooks
//! @c4-technology Rust
//! @c4-tags validation,linter

pub mod config;
pub mod error;
pub mod formatter;
pub mod rule_id;
pub mod rules;
pub mod validator;
pub mod workspace;

pub use config::LinterConfig;
pub use error::LinterError;
pub use formatter::Format;
pub use validator::Linter;

use std::path::PathBuf;
use txtx_core::validation::ValidationResult;

/// Run the linter with the specified configuration.
///
/// This is the main entry point for linting operations. It will either lint
/// a specific runbook or all runbooks in the workspace depending on configuration.
///
/// # Arguments
///
/// * `manifest_path` - Optional path to the txtx manifest file
/// * `runbook` - Optional runbook name to lint (if None, lints all)
/// * `environment` - Optional environment name for input resolution
/// * `cli_inputs` - CLI-provided input overrides
/// * `format` - Output format for validation results
///
/// # Errors
///
/// Returns `LinterError` if:
/// - The manifest cannot be loaded
/// - The specified runbook is not found
/// - Validation fails
pub fn run_linter(
    manifest_path: Option<PathBuf>,
    runbook: Option<String>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
    format: Format,
) -> Result<(), LinterError> {
    let config = LinterConfig::new(
        manifest_path,
        runbook,
        environment,
        cli_inputs,
        format,
    );

    let linter = Linter::new(&config)?;

    match config.runbook {
        Some(ref name) => linter.lint_runbook(name),
        None => linter.lint_all(),
    }
}

/// Lint runbook content directly without a workspace context.
///
/// This is a convenience function for programmatic usage when you have
/// runbook content as a string and want to validate it.
///
/// # Arguments
///
/// * `content` - The runbook content to validate
/// * `file_path` - Path for error reporting
/// * `manifest_path` - Optional manifest for input resolution
/// * `environment` - Optional environment for input resolution
///
/// # Returns
///
/// A `ValidationResult` containing any errors and warnings found.
pub fn lint_content(
    content: &str,
    file_path: &str,
    manifest_path: Option<PathBuf>,
    environment: Option<String>,
) -> ValidationResult {
    let linter = Linter::with_defaults();
    linter.validate_content(content, file_path, manifest_path.as_ref(), environment.as_ref())
}