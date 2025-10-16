//! HCL-based validation for the lint command using hcl-edit
//!
//! # C4 Architecture Annotations
//! @c4-component HCL Validator
//! @c4-container Validation Core
//! @c4-description Validates HCL syntax, block structure, and references
//! @c4-technology Rust (hcl-edit)
//! @c4-responsibility Two-phase validation: collect definitions, then validate references
//! @c4-responsibility Detect circular dependencies in variables and actions
//! @c4-responsibility Validate action outputs, signers, variables, and flow inputs
//!
//! This module uses hcl-edit's visitor pattern to perform comprehensive
//! validation of runbook files, replacing the Tree-sitter based approach.
//!
//! ## Features
//!
//! - **Two-phase validation**: Collection phase gathers all definitions, validation phase checks references
//! - **Circular dependency detection**: Detects cycles in variable and action dependencies
//! - **Reference validation**: Validates action outputs, signers, variables, and flow inputs
//! - **Addon integration**: Validates action parameters against addon specifications
//! - **Precise error reporting**: Span-based error locations with line/column information

mod dependency_graph;
mod block_processors;
mod visitor;
mod validation_helpers;

#[cfg(test)]
mod tests;

pub use visitor::{BasicHclValidator, FullHclValidator, validate_with_hcl, validate_with_hcl_and_addons};

// Re-export for tests
#[cfg(test)]
pub(crate) use visitor::HclValidationVisitor;