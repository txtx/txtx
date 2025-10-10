use std::sync::Arc;
use txtx_addon_kit::hcl::{
    expr::{Expression, Traversal, TraversalOperator},
    structure::{Attribute, Block, BlockLabel, Body},
    visit::{visit_block, visit_expr, Visit},
    Span,
};
use super::location::{SourceLocation, SourceMapper, BlockContext};

/// A comprehensive item collected from a runbook
#[derive(Debug, Clone)]
pub enum RunbookItem {
    // High-level domain-specific items
    InputReference {
        name: String,
        full_path: String,
        location: SourceLocation,
        raw: Expression,
    },
    VariableReference {
        name: String,
        full_path: String,
        location: SourceLocation,
    },
    ActionReference {
        action_name: String,
        field: Option<String>,
        full_path: String,
        location: SourceLocation,
    },
    SignerReference {
        name: String,
        full_path: String,
        location: SourceLocation,
    },
    VariableDef {
        name: String,
        location: SourceLocation,
        raw: Block,
    },
    ActionDef {
        name: String,
        action_type: String,
        namespace: String,
        action_name: String,
        location: SourceLocation,
        raw: Block,
    },
    SignerDef {
        name: String,
        signer_type: String,
        location: SourceLocation,
        raw: Block,
    },
    OutputDef {
        name: String,
        location: SourceLocation,
        raw: Block,
    },
    FlowDef {
        name: String,
        location: SourceLocation,
        raw: Block,
    },

    // Attribute-level items
    Attribute {
        key: String,
        value: Expression,
        parent_context: BlockContext,
        location: SourceLocation,
        raw: Attribute,
    },

    // Raw items for unforeseen patterns
    RawBlock {
        block_type: String,
        labels: Vec<String>,
        location: SourceLocation,
        raw: Block,
    },
    RawExpression {
        location: SourceLocation,
        raw: Expression,
    },
}


/// Collects all items from a runbook in a single pass
pub struct RunbookCollector {
    items: Vec<RunbookItem>,
    source: Arc<String>,
    file_path: String,
    current_context: Option<BlockContext>,
}

impl RunbookCollector {
    pub fn new(source: String, file_path: String) -> Self {
        Self { items: Vec::new(), source: Arc::new(source), file_path, current_context: None }
    }

    /// Collect all items from the runbook
    pub fn collect(mut self, body: &Body) -> RunbookItems {
        self.visit_body(body);
        RunbookItems { items: self.items, source: self.source, file_path: self.file_path }
    }

    fn make_location(&self, span: Option<std::ops::Range<usize>>) -> SourceLocation {
        let mapper = SourceMapper::new(&self.source);
        mapper.optional_span_to_location(span.as_ref(), self.file_path.clone())
    }

    /// Generic helper for extracting reference information from traversals
    fn extract_reference_info(
        &self,
        traversal: &Traversal,
        expected_roots: &[&str],
        max_fields: usize,
    ) -> Option<(String, Vec<String>, String)> {
        // Get the root variable
        let root = traversal.expr.as_variable()?;
        let root_str = root.as_str();

        // Check if root matches expected
        if !expected_roots.contains(&root_str) {
            return None;
        }

        // Build the full path and extract field names
        let mut path_parts = vec![root_str.to_string()];
        let mut fields = Vec::new();

        for (i, op) in traversal.operators.iter().enumerate() {
            if let TraversalOperator::GetAttr(ident) = op.value() {
                let part = ident.as_str();
                path_parts.push(part.to_string());
                if i < max_fields {
                    fields.push(part.to_string());
                }
            }
        }

        // First field is required
        if let Some(first) = fields.first() {
            Some((first.clone(), fields, path_parts.join(".")))
        } else {
            None
        }
    }

    fn extract_input_reference(&self, traversal: &Traversal) -> Option<(String, String)> {
        self.extract_reference_info(traversal, &["input"], 1).map(|(name, _, path)| (name, path))
    }

    fn extract_variable_reference(&self, traversal: &Traversal) -> Option<(String, String)> {
        self.extract_reference_info(traversal, &["var", "variable"], 1)
            .map(|(name, _, path)| (name, path))
    }

    fn extract_action_reference(
        &self,
        traversal: &Traversal,
    ) -> Option<(String, Option<String>, String)> {
        self.extract_reference_info(traversal, &["action"], 2).map(|(name, fields, path)| {
            let field = fields.get(1).cloned();
            (name, field, path)
        })
    }

    fn extract_signer_reference(&self, traversal: &Traversal) -> Option<(String, String)> {
        self.extract_reference_info(traversal, &["signer"], 1).map(|(name, _, path)| (name, path))
    }
}

impl Visit for RunbookCollector {
    fn visit_block(&mut self, block: &Block) {
        let block_type = block.ident.as_str();
        let labels: Vec<String> = block
            .labels
            .iter()
            .filter_map(|l| {
                if let BlockLabel::String(s) = l {
                    Some(s.value().to_string())
                } else {
                    None
                }
            })
            .collect();

        let location = self.make_location(block.span());

        // Create high-level items based on block type
        let item = match block_type {
            "variable" if !labels.is_empty() => {
                self.current_context = Some(BlockContext::Variable(labels[0].clone()));
                RunbookItem::VariableDef {
                    name: labels[0].clone(),
                    location: location.clone(),
                    raw: block.clone(),
                }
            }
            "action" if labels.len() >= 2 => {
                self.current_context = Some(BlockContext::Action(labels[0].clone()));
                let action_type = &labels[1];
                let (namespace, action_name) =
                    action_type.split_once("::").unwrap_or(("unknown", action_type.as_str()));

                RunbookItem::ActionDef {
                    name: labels[0].clone(),
                    action_type: action_type.clone(),
                    namespace: namespace.to_string(),
                    action_name: action_name.to_string(),
                    location: location.clone(),
                    raw: block.clone(),
                }
            }
            "signer" if labels.len() >= 2 => {
                self.current_context = Some(BlockContext::Signer(labels[0].clone()));
                RunbookItem::SignerDef {
                    name: labels[0].clone(),
                    signer_type: labels[1].clone(),
                    location: location.clone(),
                    raw: block.clone(),
                }
            }
            "output" if !labels.is_empty() => {
                self.current_context = Some(BlockContext::Output(labels[0].clone()));
                RunbookItem::OutputDef {
                    name: labels[0].clone(),
                    location: location.clone(),
                    raw: block.clone(),
                }
            }
            "flow" if !labels.is_empty() => {
                self.current_context = Some(BlockContext::Flow(labels[0].clone()));
                RunbookItem::FlowDef {
                    name: labels[0].clone(),
                    location: location.clone(),
                    raw: block.clone(),
                }
            }
            _ => {
                // Unknown or addon blocks
                RunbookItem::RawBlock {
                    block_type: block_type.to_string(),
                    labels,
                    location,
                    raw: block.clone(),
                }
            }
        };

        self.items.push(item);

        // Continue visiting children
        visit_block(self, block);

        // Reset context after block
        self.current_context = None;
    }

    fn visit_attr(&mut self, attr: &Attribute) {
        let location = self.make_location(attr.span());

        self.items.push(RunbookItem::Attribute {
            key: attr.key.as_str().to_string(),
            value: attr.value.clone(),
            parent_context: self.current_context.clone().unwrap_or(BlockContext::Unknown),
            location,
            raw: attr.clone(),
        });

        // Continue visiting the expression
        self.visit_expr(&attr.value);
    }

    fn visit_expr(&mut self, expr: &Expression) {
        let location = self.make_location(expr.span());

        // Check for various types of references
        if let Expression::Traversal(traversal) = expr {
            // Check for input references
            if let Some((name, full_path)) = self.extract_input_reference(traversal) {
                self.items.push(RunbookItem::InputReference {
                    name,
                    full_path,
                    location: location.clone(),
                    raw: expr.clone(),
                });
            }
            // Check for variable references
            else if let Some((name, full_path)) = self.extract_variable_reference(traversal) {
                self.items.push(RunbookItem::VariableReference {
                    name,
                    full_path,
                    location: location.clone(),
                });
            }
            // Check for action references
            else if let Some((action_name, field, full_path)) =
                self.extract_action_reference(traversal)
            {
                self.items.push(RunbookItem::ActionReference {
                    action_name,
                    field,
                    full_path,
                    location: location.clone(),
                });
            }
            // Check for signer references
            else if let Some((name, full_path)) = self.extract_signer_reference(traversal) {
                self.items.push(RunbookItem::SignerReference {
                    name,
                    full_path,
                    location: location.clone(),
                });
            }
        }

        // Store raw expression for unforeseen patterns
        self.items.push(RunbookItem::RawExpression { location, raw: expr.clone() });

        // Continue visiting nested expressions
        visit_expr(self, expr);
    }
}

/// Collection of runbook items with convenience methods
pub struct RunbookItems {
    items: Vec<RunbookItem>,
    #[allow(dead_code)]
    source: Arc<String>,
    #[allow(dead_code)]
    file_path: String,
}

impl RunbookItems {
    /// Get all items
    pub fn all(&self) -> &[RunbookItem] {
        &self.items
    }

    /// Generic helper for filtering items by type
    fn filter_items<'a, T, F>(&'a self, filter_fn: F) -> impl Iterator<Item = T> + 'a
    where
        T: 'a,
        F: Fn(&'a RunbookItem) -> Option<T> + 'a,
    {
        self.items.iter().filter_map(filter_fn)
    }

    /// Get only input references
    pub fn input_references(&self) -> impl Iterator<Item = (&str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::InputReference { name, location, .. } = item {
                Some((name.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Get only action definitions
    pub fn actions(&self) -> impl Iterator<Item = (&str, &str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::ActionDef { name, action_type, location, .. } = item {
                Some((name.as_str(), action_type.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Get attributes in a specific context
    pub fn attributes_in_context<'a>(
        &'a self,
        context_name: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Expression, &'a SourceLocation)> + 'a {
        self.items.iter().filter_map(move |item| {
            if let RunbookItem::Attribute { key, value, parent_context, location, .. } = item {
                parent_context
                    .name()
                    .filter(|&name| name == context_name)
                    .map(|_| (key.as_str(), value, location))
            } else {
                None
            }
        })
    }

    /// Get potentially sensitive attributes
    pub fn sensitive_attributes(
        &self,
    ) -> impl Iterator<Item = (&str, &Expression, &SourceLocation)> + '_ {
        const SENSITIVE_PATTERNS: &[&str] =
            &["secret", "key", "token", "password", "credential", "private"];

        self.items.iter().filter_map(|item| {
            if let RunbookItem::Attribute { key, value, location, .. } = item {
                let key_lower = key.to_lowercase();
                if SENSITIVE_PATTERNS.iter().any(|pattern| key_lower.contains(pattern)) {
                    Some((key.as_str(), value, location))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Check if an input is defined in variables
    pub fn is_input_defined(&self, input_name: &str) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, RunbookItem::VariableDef { name, .. } if name == input_name))
    }

    /// Get all variable definitions
    pub fn variables(&self) -> impl Iterator<Item = (&str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::VariableDef { name, location, .. } = item {
                Some((name.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Get all signer definitions
    pub fn signers(&self) -> impl Iterator<Item = (&str, &str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::SignerDef { name, signer_type, location, .. } = item {
                Some((name.as_str(), signer_type.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Access to underlying items for custom filtering
    pub fn iter(&self) -> impl Iterator<Item = &RunbookItem> {
        self.items.iter()
    }

    /// Get all variable references (var.* or variable.*)
    pub fn variable_references(&self) -> impl Iterator<Item = (&str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::VariableReference { name, location, .. } = item {
                Some((name.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Get all action references (action.*)
    pub fn action_references(&self) -> impl Iterator<Item = (&str, Option<&str>, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::ActionReference { action_name, field, location, .. } = item {
                Some((action_name.as_str(), field.as_deref(), location))
            } else {
                None
            }
        })
    }

    /// Get all signer references (signer.* references and signer attributes)
    pub fn signer_references(&self) -> impl Iterator<Item = (&str, &SourceLocation)> + '_ {
        self.items.iter().filter_map(|item| match item {
            RunbookItem::SignerReference { name, location, .. } => Some((name.as_str(), location)),
            RunbookItem::Attribute { key, value, location, .. } if key == "signer" => {
                if let Expression::String(s) = value {
                    Some((s.as_str(), location))
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    /// Get all outputs
    pub fn outputs(&self) -> impl Iterator<Item = (&str, &SourceLocation)> + '_ {
        self.filter_items(move |item| {
            if let RunbookItem::OutputDef { name, location, .. } = item {
                Some((name.as_str(), location))
            } else {
                None
            }
        })
    }

    /// Convert to owned vector
    pub fn into_vec(self) -> Vec<RunbookItem> {
        self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_collector_basic() {
        let content = r#"
        variable "my_input" {
            default = "value"
        }

        action "my_action" "evm::call" {
            contract = "0x123"
        }

        signer "my_signer" "evm" {
            mnemonic = input.MNEMONIC
        }
        "#;

        let body = Body::from_str(content).unwrap();
        let collector = RunbookCollector::new(content.to_string(), "test.tx".to_string());
        let items = collector.collect(&body);

        // Check variables were collected
        let vars: Vec<_> = items.variables().collect();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].0, "my_input");

        // Check actions were collected
        let actions: Vec<_> = items.actions().collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].0, "my_action");
        assert_eq!(actions[0].1, "evm::call");

        // Check signers were collected
        let signers: Vec<_> = items.signers().collect();
        assert_eq!(signers.len(), 1);
        assert_eq!(signers[0].0, "my_signer");
        assert_eq!(signers[0].1, "evm");

        // Check input references were collected
        let inputs: Vec<_> = items.input_references().collect();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].0, "MNEMONIC");
    }
}
