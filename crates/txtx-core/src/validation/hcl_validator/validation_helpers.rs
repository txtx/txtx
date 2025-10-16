//! Shared validation helper functions for HCL validation.
//!
//! This module contains validation logic that is shared between the collection
//! phase (block_processors) and the validation phase (visitor).

use std::collections::HashMap;
use txtx_addon_kit::constants::{
    DEPENDS_ON, DESCRIPTION, MARKDOWN, MARKDOWN_FILEPATH, POST_CONDITION, PRE_CONDITION,
};

use crate::kit::types::commands::CommandSpecification;
use super::visitor::ValidationError;

/// Validate action format (namespace::action)
pub fn validate_action_format(action: &str) -> Result<(&str, &str), ValidationError> {
    action.split_once("::").ok_or_else(|| ValidationError::InvalidFormat {
        value: action.to_string(),
        expected: "namespace::action",
    })
}

/// Check if namespace exists
pub fn validate_namespace_exists<'a>(
    namespace: &str,
    specs: &'a HashMap<String, Vec<(String, CommandSpecification)>>,
) -> Result<&'a Vec<(String, CommandSpecification)>, ValidationError> {
    specs.get(namespace).ok_or_else(|| ValidationError::UnknownNamespace {
        namespace: namespace.to_string(),
        available: specs.keys().cloned().collect(),
    })
}

/// Find action in namespace
pub fn find_action_spec<'a>(
    action: &str,
    namespace_actions: &'a [(String, CommandSpecification)],
) -> Option<&'a CommandSpecification> {
    namespace_actions.iter().find(|(matcher, _)| matcher == action).map(|(_, spec)| spec)
}

/// Validate a complete action
pub fn validate_action(
    action_type: &str,
    specs: &HashMap<String, Vec<(String, CommandSpecification)>>,
) -> Result<CommandSpecification, ValidationError> {
    let (namespace, action) = validate_action_format(action_type)?;
    let namespace_actions = validate_namespace_exists(namespace, specs)?;

    find_action_spec(action, namespace_actions).cloned().ok_or_else(|| {
        ValidationError::UnknownAction {
            namespace: namespace.to_string(),
            action: action.to_string(),
            cause: None,
        }
    })
}

/// Check if an attribute is an inherited property
pub fn is_inherited_property(attr_name: &str) -> bool {
    matches!(
        attr_name,
        MARKDOWN | MARKDOWN_FILEPATH | DESCRIPTION | DEPENDS_ON | PRE_CONDITION | POST_CONDITION
    )
}
