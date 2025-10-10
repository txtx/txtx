//! Dependency extraction from txtx HCL content.
//!
//! Analyzes txtx runbook content to extract references to:
//! - `input.*` (manifest inputs)
//! - `output.*` (action outputs)
//! - `variable.*` (variables from other files)

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Dependencies extracted from a document.
#[derive(Debug, Clone, Default)]
pub struct ExtractedDependencies {
    /// References to manifest inputs (input.*)
    pub uses_manifest_inputs: bool,
    /// Action names referenced via output.*
    pub action_outputs: HashSet<String>,
    /// Variable names referenced via variable.*
    pub variables: HashSet<String>,
    /// Action names defined in this document
    pub defined_actions: HashSet<String>,
    /// Variable names defined in this document
    pub defined_variables: HashSet<String>,
}

impl ExtractedDependencies {
    /// Creates an empty set of dependencies.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if any dependencies were found.
    pub fn is_empty(&self) -> bool {
        !self.uses_manifest_inputs
            && self.action_outputs.is_empty()
            && self.variables.is_empty()
            && self.defined_actions.is_empty()
            && self.defined_variables.is_empty()
    }
}

/// Helper to extract capture group 1 into a HashSet.
fn extract_captures_to_set(regex: &Regex, content: &str) -> HashSet<String> {
    regex
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Extracts dependencies from txtx HCL content.
///
/// Scans the content for:
/// - `input.something` - indicates dependency on manifest
/// - `output.action_name.field` - indicates dependency on another action
/// - `variable.var_name` - indicates dependency on another variable
/// - `action "name" ...` - action definitions
/// - `variable "name" ...` - variable definitions
///
/// # Arguments
///
/// * `content` - The HCL content to analyze
///
/// # Returns
///
/// Extracted dependencies found in the content.
pub fn extract_dependencies(content: &str) -> ExtractedDependencies {
    static INPUT_REGEX: OnceLock<Regex> = OnceLock::new();
    static OUTPUT_REGEX: OnceLock<Regex> = OnceLock::new();
    static VARIABLE_REF_REGEX: OnceLock<Regex> = OnceLock::new();
    static ACTION_DEF_REGEX: OnceLock<Regex> = OnceLock::new();
    static VARIABLE_DEF_REGEX: OnceLock<Regex> = OnceLock::new();

    let input_re = INPUT_REGEX.get_or_init(|| Regex::new(r"\binput\.\w+").unwrap());
    let output_re = OUTPUT_REGEX.get_or_init(|| Regex::new(r"\boutput\.(\w+)").unwrap());
    let variable_ref_re =
        VARIABLE_REF_REGEX.get_or_init(|| Regex::new(r"\bvariable\.(\w+)").unwrap());
    let action_def_re = ACTION_DEF_REGEX.get_or_init(|| Regex::new(r#"action\s+"(\w+)""#).unwrap());
    let variable_def_re =
        VARIABLE_DEF_REGEX.get_or_init(|| Regex::new(r#"variable\s+"(\w+)""#).unwrap());

    ExtractedDependencies {
        uses_manifest_inputs: input_re.is_match(content),
        action_outputs: extract_captures_to_set(output_re, content),
        variables: extract_captures_to_set(variable_ref_re, content),
        defined_actions: extract_captures_to_set(action_def_re, content),
        defined_variables: extract_captures_to_set(variable_def_re, content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_manifest_input_dependency() {
        let content = r#"
variable "key" {
    value = input.api_key
}
"#;
        let deps = extract_dependencies(content);
        assert!(deps.uses_manifest_inputs);
        assert!(deps.action_outputs.is_empty());
        assert!(deps.variables.is_empty());
    }

    #[test]
    fn test_extract_output_dependency() {
        let content = r#"
action "verify" "evm::call" {
    contract_address = output.deploy.address
}
"#;
        let deps = extract_dependencies(content);
        assert!(!deps.uses_manifest_inputs);
        assert_eq!(deps.action_outputs.len(), 1);
        assert!(deps.action_outputs.contains("deploy"));
        assert!(deps.variables.is_empty());
    }

    #[test]
    fn test_extract_variable_dependency() {
        let content = r#"
variable "full_url" {
    value = "${variable.base_url}/v1/endpoint"
}
"#;
        let deps = extract_dependencies(content);
        assert!(!deps.uses_manifest_inputs);
        assert!(deps.action_outputs.is_empty());
        assert_eq!(deps.variables.len(), 1);
        assert!(deps.variables.contains("base_url"));
    }

    #[test]
    fn test_extract_multiple_dependencies() {
        let content = r#"
variable "derived" {
    value = "${input.api_key}_${variable.base}"
}
"#;
        let deps = extract_dependencies(content);
        assert!(deps.uses_manifest_inputs);
        assert!(deps.action_outputs.is_empty());
        assert_eq!(deps.variables.len(), 1);
        assert!(deps.variables.contains("base"));
    }

    #[test]
    fn test_no_dependencies() {
        let content = r#"
action "deploy" "evm::call" {
    contract_address = "0x123"
}
"#;
        let deps = extract_dependencies(content);
        // Should have defined_actions but no dependency references
        assert!(!deps.uses_manifest_inputs);
        assert!(deps.action_outputs.is_empty());
        assert!(deps.variables.is_empty());
        assert_eq!(deps.defined_actions.len(), 1);
        assert!(deps.defined_actions.contains("deploy"));
    }

    #[test]
    fn test_multiple_output_references() {
        let content = r#"
action "final" "evm::call" {
    address1 = output.deploy.address
    address2 = output.verify.result
    status = output.deploy.status
}
"#;
        let deps = extract_dependencies(content);
        assert_eq!(deps.action_outputs.len(), 2);
        assert!(deps.action_outputs.contains("deploy"));
        assert!(deps.action_outputs.contains("verify"));
    }

    #[test]
    fn test_multiple_variable_references() {
        let content = r#"
variable "combined" {
    value = "${variable.a}_${variable.b}_${variable.c}"
}
"#;
        let deps = extract_dependencies(content);
        assert_eq!(deps.variables.len(), 3);
        assert!(deps.variables.contains("a"));
        assert!(deps.variables.contains("b"));
        assert!(deps.variables.contains("c"));
    }
}
