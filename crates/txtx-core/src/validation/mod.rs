//! Shared validation module for runbook files
//!
//! This module provides validation functionality that is shared between
//! the lint command (CLI) and the LSP for real-time error detection.
//!
//! # C4 Architecture Annotations
//! @c4-container Validation Core
//! @c4-description Core validation logic shared between CLI and LSP
//! @c4-technology Rust (txtx-core)

pub mod context;
pub mod file_boundary;
pub mod linter_rules;
pub mod hcl_diagnostics;
pub mod hcl_validator;
pub mod manifest_validator;
pub mod rule_id;
pub mod types;
pub mod validator;

pub use context::{ValidationContext, ValidationContextBuilder, ValidationContextExt};
pub use linter_rules::{
    get_linter_rules, get_strict_linter_rules, CliInputOverrideRule, InputNamingConventionRule,
    SensitiveDataRule,
};
pub use manifest_validator::{
    validate_inputs_against_manifest, ManifestValidationConfig, ManifestValidationContext,
    ManifestValidationRule, ValidationOutcome,
};
pub use rule_id::{AddonScope, CoreRuleId, RuleIdentifier};
pub use file_boundary::FileBoundaryMap;
pub use types::{
    LocatedInputRef, ValidationError, ValidationResult, ValidationSuggestion, ValidationWarning,
};
pub use validator::{validate_runbook, ValidatorConfig};
