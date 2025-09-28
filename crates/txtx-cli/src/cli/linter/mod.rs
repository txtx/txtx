//! Linter for txtx runbooks
//!
//! # C4 Architecture Annotations
//! @c4-component Linter Engine
//! @c4-container txtx-cli
//! @c4-description Orchestrates validation pipeline for runbooks
//! @c4-technology Rust
//! @c4-tags validation,linter

pub mod config;
pub mod formatter;
pub mod rule_id;
pub mod rules;
pub mod validator;
pub mod workspace;

pub use config::LinterConfig;
pub use formatter::Format;
pub use validator::Linter;

use std::path::PathBuf;
use txtx_core::validation::ValidationResult;

#[allow(dead_code)] // May be used in future CLI commands
pub fn run_linter(
    manifest_path: Option<PathBuf>,
    runbook: Option<String>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
    format: Format,
) -> Result<(), String> {
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

#[allow(dead_code)] // Public API for programmatic usage
pub fn lint_content(
    content: &str,
    file_path: &str,
    manifest_path: Option<PathBuf>,
    environment: Option<String>,
) -> ValidationResult {
    let linter = Linter::with_defaults();
    linter.validate_content(content, file_path, manifest_path.as_ref(), environment.as_ref())
}