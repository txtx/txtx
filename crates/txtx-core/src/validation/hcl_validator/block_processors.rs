//! Block processing for HCL validation.

use std::collections::HashMap;

use txtx_addon_kit::hcl::{structure::{Block, BlockLabel}, Span};

use crate::kit::types::commands::CommandSpecification;
use crate::runbook::location::SourceMapper;
use crate::types::ConstructType;
use crate::validation::hcl_validator::visitor::{
    CollectedItem, DefinitionItem, DeclarationItem, BlockType, Position,
    ValidationError,
};
use super::validation_helpers;

/// Extract position from a block's identifier span
///
/// Converts the block's identifier span to a Position using the source mapper.
/// Returns a default position (1, 1) if the block has no span information.
///
/// This helper consolidates the repeated pattern of extracting positions
/// from block identifiers, reducing code duplication across processor functions.
fn extract_block_position(block: &Block, source_mapper: &SourceMapper) -> Position {
    block.ident.span()
        .as_ref()
        .map(|span| {
            let (line, col) = source_mapper.span_to_position(span);
            Position::new(line, col)
        })
        .unwrap_or_default()
}

/// Process a block during the collection phase.
pub fn process_block(
    block: &Block,
    block_type: BlockType,
    addon_specs: &HashMap<String, Vec<(String, CommandSpecification)>>,
    source_mapper: &SourceMapper,
) -> Result<Vec<CollectedItem>, ValidationError> {
    match block_type {
        BlockType::Signer => process_signer(block, source_mapper),
        BlockType::Variable => process_variable(block, source_mapper),
        BlockType::Output => process_output(block),
        BlockType::Action => process_action(block, addon_specs, source_mapper),
        BlockType::Flow => process_flow(block, source_mapper),
        BlockType::Addon | BlockType::Unknown => Ok(Vec::new()),
    }
}

fn process_signer(block: &Block, source_mapper: &SourceMapper) -> Result<Vec<CollectedItem>, ValidationError> {
    let name = block.labels.extract_name()
        .ok_or(ValidationError::MissingLabel("signer name"))?;

    let signer_type = block.labels.extract_type()
        .ok_or(ValidationError::MissingLabel("signer type"))?;

    let position = extract_block_position(block, source_mapper);

    // Extract string attributes from the block body
    let mut attributes = HashMap::new();
    for attr in block.body.attributes() {
        if let Some(value) = extract_string_value(&attr.value) {
            attributes.insert(attr.key.to_string(), value);
        }
    }

    Ok(vec![
        CollectedItem::Definition(DefinitionItem::Signer {
            name: name.to_string(),
            signer_type: signer_type.to_string(),
            attributes,
            position,
        })
    ])
}

/// Extract a string value from an expression, if it is a simple string literal
fn extract_string_value(expr: &txtx_addon_kit::hcl::expr::Expression) -> Option<String> {
    use txtx_addon_kit::hcl::expr::Expression;
    match expr {
        Expression::String(s) => Some(s.value().to_string()),
        _ => None,
    }
}

fn process_variable(block: &Block, source_mapper: &SourceMapper) -> Result<Vec<CollectedItem>, ValidationError> {
    use txtx_addon_kit::hcl::visit::{visit_expr, Visit};
    use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};

    let name = block.labels.extract_name()
        .ok_or(ValidationError::MissingLabel("variable name"))?;

    let position = extract_block_position(block, source_mapper);

    // Extract dependencies from the variable's value
    let mut dependencies = Vec::new();

    struct DependencyExtractor<'a> {
        dependencies: &'a mut Vec<String>,
    }

    impl<'a> Visit for DependencyExtractor<'a> {
        fn visit_expr(&mut self, expr: &Expression) {
            // Use pattern matching to extract variable dependencies
            if let Expression::Traversal(traversal) = expr {
                traversal.expr.as_variable()
                    .filter(|name| name.as_str() == ConstructType::Variable.as_ref())
                    .and_then(|_| traversal.operators.first())
                    .and_then(|op| match op.value() {
                        TraversalOperator::GetAttr(attr) => Some(attr.to_string()),
                        _ => None,
                    })
                    .map(|dep| self.dependencies.push(dep));
            }
            visit_expr(self, expr);
        }
    }

    let mut extractor = DependencyExtractor { dependencies: &mut dependencies };
    // Visit the entire block body - the visitor will find all expressions
    extractor.visit_body(&block.body);

    Ok(vec![
        CollectedItem::Definition(DefinitionItem::Variable {
            name: name.to_string(),
            position,
        }),
        CollectedItem::Dependencies {
            entity_type: ConstructType::Variable.to_string(),
            entity_name: name.to_string(),
            depends_on: dependencies
        }
    ])
}

fn process_output(block: &Block) -> Result<Vec<CollectedItem>, ValidationError> {
    let name = block.labels.extract_name().ok_or(ValidationError::MissingLabel("output name"))?;

    Ok(vec![CollectedItem::Definition(DefinitionItem::Output(name.to_string()))])
}

fn process_action(
    block: &Block,
    addon_specs: &HashMap<String, Vec<(String, CommandSpecification)>>,
    source_mapper: &SourceMapper,
) -> Result<Vec<CollectedItem>, ValidationError> {
    use txtx_addon_kit::hcl::visit::{visit_expr, visit_block, Visit};
    use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};

    let name = block.labels.extract_name()
        .ok_or(ValidationError::MissingLabel("action name"))?;

    let action_type = block.labels.extract_type()
        .ok_or(ValidationError::MissingLabel("action type"))?;

    let position = extract_block_position(block, source_mapper);

    // Always collect the action, but validation will happen in validation phase
    // We still try to get the spec for parameter validation later
    let spec = validation_helpers::validate_action(action_type, addon_specs).ok();

    // Extract action dependencies using visitor pattern
    struct DependencyExtractor {
        dependencies: Vec<String>,
        in_post_condition: bool,
    }

    impl Visit for DependencyExtractor {
        fn visit_block(&mut self, block: &txtx_addon_kit::hcl::structure::Block) {
            // Track when entering/leaving post_condition blocks
            let was_in_post_condition = self.in_post_condition;
            if block.ident.as_str() == "post_condition" {
                self.in_post_condition = true;
            }

            // Visit the block's contents
            visit_block(self, block);

            // Restore the previous state
            self.in_post_condition = was_in_post_condition;
        }

        fn visit_expr(&mut self, expr: &Expression) {
            // Extract action dependencies using functional style
            // Skip dependencies in post_condition blocks since they execute AFTER the action
            if !self.in_post_condition {
                if let Expression::Traversal(traversal) = expr {
                    traversal.expr.as_variable()
                        .filter(|name| name.as_str() == ConstructType::Action.as_ref())
                        .and_then(|_| traversal.operators.first())
                        .and_then(|op| match op.value() {
                            TraversalOperator::GetAttr(name) => Some(name.to_string()),
                            _ => None,
                        })
                        .map(|dep| self.dependencies.push(dep));
                }
            }
            visit_expr(self, expr);
        }
    }

    let mut extractor = DependencyExtractor {
        dependencies: Vec::new(),
        in_post_condition: false,
    };
    // Visit the entire block body - the visitor will find all expressions
    extractor.visit_body(&block.body);

    let mut items = vec![
        CollectedItem::Declaration(DeclarationItem::Action {
            name: name.to_string(),
            action_type: action_type.to_string(),
            spec,
            position,
        })
    ];

    if !extractor.dependencies.is_empty() {
        items.push(CollectedItem::Dependencies {
            entity_type: ConstructType::Action.to_string(),
            entity_name: name.to_string(),
            depends_on: extractor.dependencies,
        });
    }

    Ok(items)
}

fn process_flow(
    block: &Block,
    source_mapper: &SourceMapper,
) -> Result<Vec<CollectedItem>, ValidationError> {
    let name = block.labels.extract_name()
        .ok_or(ValidationError::MissingLabel("flow name"))?;

    let inputs: Vec<String> = block.body
        .attributes()
        .filter(|attr| !validation_helpers::is_inherited_property(attr.key.as_str()))
        .map(|attr| attr.key.to_string())
        .collect();

    let position = extract_block_position(block, source_mapper);

    Ok(vec![
        CollectedItem::Declaration(DeclarationItem::Flow {
            name: name.to_string(),
            inputs,
            position,
        })
    ])
}


trait BlockLabelExt {
    fn extract_name(&self) -> Option<&str>;
    fn extract_type(&self) -> Option<&str>;
}

impl BlockLabelExt for [BlockLabel] {
    fn extract_name(&self) -> Option<&str> {
        self.get(0).and_then(|label| match label {
            BlockLabel::String(s) => Some(s.value().as_str()),
            _ => None,
        })
    }

    fn extract_type(&self) -> Option<&str> {
        self.get(1).and_then(|label| match label {
            BlockLabel::String(s) => Some(s.value().as_str()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::validation_helpers::is_inherited_property;

    #[test]
    fn test_is_inherited_property() {
        assert!(is_inherited_property("description"));
        assert!(is_inherited_property("markdown"));
        assert!(is_inherited_property("markdown_filepath"));
        assert!(is_inherited_property("depends_on"));
        assert!(is_inherited_property("pre_condition"));
        assert!(is_inherited_property("post_condition"));
        assert!(!is_inherited_property("name"));
        assert!(!is_inherited_property("value"));
    }
}