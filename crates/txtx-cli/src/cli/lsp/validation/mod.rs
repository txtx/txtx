//! LSP validation integration with linter validation rules
//!
//! This module bridges the linter validation framework with LSP diagnostics,
//! allowing us to reuse the same validation logic for real-time feedback.

mod adapter;
mod converter;
mod hcl_converter;

pub use adapter::LinterValidationAdapter;
pub use hcl_converter::validation_errors_to_diagnostics;
