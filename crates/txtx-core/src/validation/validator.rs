//! High-level validation API for runbook files

use super::hcl_validator::HclValidationVisitor;
use super::types::ValidationResult;
use crate::kit::hcl::structure::Body;
use crate::kit::types::commands::{CommandSpecification, PreCommandSpecification};
use std::collections::HashMap;

/// Configuration for the validator
pub struct ValidatorConfig {
    /// Addon specifications for validation
    pub addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
}

impl ValidatorConfig {
    pub fn new() -> Self {
        Self { addon_specs: HashMap::new() }
    }

    /// Add specifications from an addon
    pub fn add_addon_specs(&mut self, namespace: String, specs: Vec<PreCommandSpecification>) {
        let actions = specs
            .into_iter()
            .filter_map(|a| match a {
                PreCommandSpecification::Atomic(spec) => Some((spec.matcher.clone(), spec)),
                _ => None,
            })
            .collect();
        self.addon_specs.insert(namespace, actions);
    }
}

/// Validate a runbook file
pub fn validate_runbook(
    file_path: &str,
    source: &str,
    body: &Body,
    _config: ValidatorConfig,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    let mut visitor = HclValidationVisitor::new(&mut result, file_path, source);

    // TODO: Need to update HclValidationVisitor to accept addon_specs as parameter
    // For now, it uses get_addon_specifications() internally

    visitor.validate(body);

    result
}
