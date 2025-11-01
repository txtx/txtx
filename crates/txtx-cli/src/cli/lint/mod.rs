//! Linter module for txtx runbooks
//!
//! This module provides linting functionality for txtx runbooks,
//! including validation rules, formatting, and workspace analysis.

// Submodules
mod command;
mod config;
mod error;
mod formatter;
mod rule_id;
mod rules;
mod validator;
pub mod workspace;

// Re-export main components
pub use command::{run_lint, LinterOptions};
pub use config::LinterConfig;
pub use error::LinterError;
pub use formatter::Format;
pub use rule_id::CliRuleId;
pub use txtx_core::validation::CoreRuleId;
pub use validator::Linter;
pub use workspace::WorkspaceAnalyzer;

