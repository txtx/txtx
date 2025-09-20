//! Shared validation module for runbook files
//!
//! This module provides validation functionality that is shared between
//! the doctor command (CLI) and the LSP for real-time error detection.

pub mod context;
pub mod doctor_rules;
pub mod hcl_diagnostics;
pub mod hcl_validator;
pub mod manifest_validator;
pub mod types;
pub mod validator;

pub use context::{ValidationContext, ValidationContextBuilder, ValidationContextExt};
pub use doctor_rules::{
    get_doctor_rules, get_strict_doctor_rules, CliInputOverrideRule, InputNamingConventionRule,
    SensitiveDataRule,
};
pub use manifest_validator::{
    validate_inputs_against_manifest, ManifestValidationConfig, ManifestValidationContext,
    ManifestValidationRule, ValidationOutcome,
};
pub use types::{
    LocatedInputRef, ValidationError, ValidationResult, ValidationSuggestion, ValidationWarning,
};
pub use validator::{validate_runbook, ValidatorConfig};
