//! Diagnostic conversion and validation utilities
//!
//! This module provides unified conversion from validation diagnostics
//! to LSP diagnostic format, as well as validation providers.

pub mod converter;
pub mod provider;

pub use converter::to_lsp_diagnostic;
pub use provider::{validate_runbook, validate_workspace};
