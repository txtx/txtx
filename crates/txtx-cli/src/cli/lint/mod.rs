//! Linter module for txtx runbooks
//!
//! This module provides validation and formatting capabilities for txtx runbooks and manifests.
//!
//! **NOTE: Configuration support via `.txtxlint.yml` is EXPERIMENTAL and subject to change.**
//!
//! # Architecture
//!
//! The linter consists of several key components:
//! - **Validator** (`Linter`): Main entry point for validation
//! - **Rules** (`rules.rs`): Validation functions for different checks
//! - **Formatters** (`Format`): Output formatters (json, stylish, github, csv)
//! - **Workspace** (`WorkspaceAnalyzer`): Analyzes and validates entire workspaces
//!
//! # Available Validation Rules
//!
//! - **undefined-input**: Checks that all input references are defined
//! - **naming-convention**: Enforces snake_case naming for inputs
//! - **cli-override**: Warns when CLI inputs override manifest values
//! - **sensitive-data**: Detects potential sensitive data exposure in inputs

// Submodules
mod command;
mod config;
mod error;
mod formatter;
mod rule_id;
mod rules;
mod validator;
pub mod workspace;

// Test utilities (only compiled during tests)
#[cfg(test)]
#[macro_use]
pub mod test_utils;

// Re-export main components
pub use command::{run_lint, LinterOptions};
pub use config::LinterConfig;
pub use error::LinterError;
pub use formatter::Format;
pub use validator::Linter;
pub use workspace::WorkspaceAnalyzer;

