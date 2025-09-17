//! HCL-based validation for the doctor command using hcl-edit
//!
//! This module uses hcl-edit's visitor pattern to perform comprehensive
//! validation of runbook files, replacing the Tree-sitter based approach.

use std::collections::{HashMap, HashSet, VecDeque};

use txtx_addon_kit::hcl::{
    expr::{Expression, Traversal, TraversalOperator},
    structure::{Block, BlockLabel, Body},
    visit::{visit_block, visit_expr, Visit},
    Span,
};

use super::types::{LocatedInputRef, ValidationError, ValidationResult};
use crate::kit::types::commands::CommandSpecification;

/// Get documentation link for an action
fn get_action_doc_link(namespace: &str, action: &str) -> Option<String> {
    match namespace {
        "bitcoin" => Some(format!("https://docs.txtx.sh/addons/bitcoin/actions#{}", action)),
        "evm" => Some(format!("https://docs.txtx.sh/addons/evm/actions#{}", action)),
        "stacks" => Some(format!("https://docs.txtx.sh/addons/stacks/actions#{}", action)),
        "svm" => Some(format!("https://docs.txtx.sh/addons/svm/actions#{}", action)),
        "ovm" => Some(format!("https://docs.txtx.sh/addons/ovm/actions#{}", action)),
        "telegram" => Some(format!("https://docs.txtx.sh/addons/telegram/actions#{}", action)),
        _ => None,
    }
}

/// A visitor that validates HCL runbooks
pub struct HclValidationVisitor<'a> {
    /// Results collector
    result: &'a mut ValidationResult,
    /// Path to the current file being validated
    file_path: String,
    /// Source content for extracting line/column from spans
    source: &'a str,

    // === Collection Phase Data ===
    /// Map of action names to their types
    action_types: HashMap<String, String>,
    /// Map of action names to their specifications
    action_specs: HashMap<String, CommandSpecification>,
    /// Addon specifications
    addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    /// Track all variable definitions
    defined_variables: HashSet<String>,
    /// Track all signer definitions  
    defined_signers: HashMap<String, String>,
    /// Track all output definitions
    defined_outputs: HashSet<String>,
    /// Track flow definitions and their inputs
    flow_inputs: HashMap<String, Vec<String>>,
    /// Track flow block locations for error reporting
    flow_locations: HashMap<String, (usize, usize)>,

    // === Context Tracking ===
    /// Current block being processed
    current_block: Option<BlockContext>,
    /// Whether we're in validation phase (vs collection phase)
    is_validation_phase: bool,
    /// Collected input references
    pub input_refs: Vec<LocatedInputRef>,

    // === Error Tracking ===
    /// Blocks that have errors (to skip in validation phase)
    blocks_with_errors: HashSet<String>,
    /// Primary errors (namespace/type errors) to report first
    primary_errors: Vec<ValidationError>,
}

#[derive(Clone, Debug)]
struct BlockContext {
    block_type: String,
    name: String,
    span: Option<std::ops::Range<usize>>,
}

impl<'a> HclValidationVisitor<'a> {
    pub fn new(result: &'a mut ValidationResult, file_path: &str, source: &'a str) -> Self {
        Self {
            result,
            file_path: file_path.to_string(),
            source,
            action_types: HashMap::new(),
            action_specs: HashMap::new(),
            addon_specs: HashMap::new(), // Default to empty, use new_with_addons for real specs
            defined_variables: HashSet::new(),
            defined_signers: HashMap::new(),
            defined_outputs: HashSet::new(),
            flow_inputs: HashMap::new(),
            flow_locations: HashMap::new(),
            current_block: None,
            is_validation_phase: false,
            input_refs: Vec::new(),
            blocks_with_errors: HashSet::new(),
            primary_errors: Vec::new(),
        }
    }

    pub fn new_with_addons(
        result: &'a mut ValidationResult,
        file_path: &str,
        source: &'a str,
        addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        Self {
            result,
            file_path: file_path.to_string(),
            source,
            action_types: HashMap::new(),
            action_specs: HashMap::new(),
            addon_specs,
            defined_variables: HashSet::new(),
            defined_signers: HashMap::new(),
            defined_outputs: HashSet::new(),
            flow_inputs: HashMap::new(),
            flow_locations: HashMap::new(),
            current_block: None,
            is_validation_phase: false,
            input_refs: Vec::new(),
            blocks_with_errors: HashSet::new(),
            primary_errors: Vec::new(),
        }
    }

    /// Convert a span to line/column position
    fn span_to_position(&self, span: &std::ops::Range<usize>) -> (usize, usize) {
        let start = span.start;
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in self.source.char_indices() {
            if i >= start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Convert an optional span to line/column position, defaulting to (0, 0)
    fn optional_span_to_position(&self, span: Option<std::ops::Range<usize>>) -> (usize, usize) {
        if let Some(span) = span {
            self.span_to_position(&span)
        } else {
            (0, 0)
        }
    }

    /// Process a block based on its type
    fn process_block(&mut self, block: &Block) {
        let block_type = block.ident.value().as_str();

        // Set current context with span
        let span = block.span();
        self.current_block = Some(BlockContext {
            block_type: block_type.to_string(),
            name: String::new(), // Will be filled based on block type
            span,
        });

        match block_type {
            "addon" => {
                // Addon blocks don't need tracking for validation
            }
            "signer" => {
                if let Some(BlockLabel::String(name)) = block.labels.get(0) {
                    if let Some(ctx) = &mut self.current_block {
                        ctx.name = name.value().to_string();
                    }

                    if !self.is_validation_phase {
                        // Collection phase: record signer
                        if let Some(BlockLabel::String(signer_type)) = block.labels.get(1) {
                            self.defined_signers
                                .insert(name.value().to_string(), signer_type.value().to_string());
                        }
                    }
                }
            }
            "action" => {
                if let Some(BlockLabel::String(name)) = block.labels.get(0) {
                    if let Some(ctx) = &mut self.current_block {
                        ctx.name = name.value().to_string();
                    }

                    if let Some(BlockLabel::String(action_type)) = block.labels.get(1) {
                        let name_str = name.value().to_string();
                        let type_str = action_type.value().to_string();

                        if !self.is_validation_phase {
                            // Collection phase: record action and its type
                            self.action_types.insert(name_str.clone(), type_str.clone());

                            // Validate and get specification for this action
                            if let Some((namespace, action_name)) = type_str.split_once("::") {
                                if let Some(addon_actions) = self.addon_specs.get(namespace) {
                                    if let Some((_, spec)) = addon_actions
                                        .iter()
                                        .find(|(matcher, _)| matcher == action_name)
                                    {
                                        self.action_specs.insert(name_str.clone(), spec.clone());
                                    } else {
                                        // Unknown action type within known namespace
                                        let (line, col) =
                                            self.optional_span_to_position(action_type.span());
                                        let available_actions: Vec<String> = addon_actions
                                            .iter()
                                            .map(|(matcher, _)| {
                                                format!("{}::{}", namespace, matcher)
                                            })
                                            .collect();

                                        // Check for common typos
                                        let suggestion = if namespace == "evm"
                                            && action_name == "deploy"
                                        {
                                            Some("Did you mean 'evm::deploy_contract'?".to_string())
                                        } else {
                                            // Find similar action names
                                            let similar = addon_actions
                                                .iter()
                                                .map(|(matcher, _)| matcher)
                                                .filter(|m| {
                                                    m.contains(action_name)
                                                        || action_name.contains(*m)
                                                })
                                                .map(|m| format!("{}::{}", namespace, m))
                                                .collect::<Vec<_>>();

                                            if !similar.is_empty() {
                                                Some(format!(
                                                    "Did you mean: {}?",
                                                    similar.join(" or ")
                                                ))
                                            } else {
                                                None
                                            }
                                        };

                                        self.primary_errors.push(ValidationError {
                                            message: format!(
                                                "Unknown action type '{}::{}'. Available actions for '{}': {}",
                                                namespace, action_name, namespace, available_actions.join(", ")
                                            ),
                                            file: self.file_path.clone(),
                                            line: if line > 0 { Some(line) } else { None },
                                            column: if col > 0 { Some(col) } else { None },
                                            context: suggestion,
                                            documentation_link: Some(format!("https://docs.txtx.sh/addons/{}/actions", namespace)),
                                        });
                                        self.blocks_with_errors.insert(name_str.clone());
                                    }
                                } else {
                                    // Unknown namespace
                                    let (line, col) =
                                        self.optional_span_to_position(action_type.span());
                                    let available_namespaces: Vec<&str> =
                                        self.addon_specs.keys().map(|s| s.as_str()).collect();

                                    self.primary_errors.push(ValidationError {
                                        message: format!(
                                            "Unknown addon namespace '{}'. Available namespaces: {}",
                                            namespace, available_namespaces.join(", ")
                                        ),
                                        file: self.file_path.clone(),
                                        line: if line > 0 { Some(line) } else { None },
                                        column: if col > 0 { Some(col) } else { None },
                                        context: Some("Make sure you have the correct addon name".to_string()),
                                        documentation_link: None,
                                    });
                                    self.blocks_with_errors.insert(name_str.clone());
                                }
                            } else {
                                // Invalid action type format (missing ::)
                                let (line, col) =
                                    self.optional_span_to_position(action_type.span());
                                self.primary_errors.push(ValidationError {
                                    message: format!("Invalid action type '{}' - must be in format 'namespace::action'", type_str),
                                    file: self.file_path.clone(),
                                    line: if line > 0 { Some(line) } else { None },
                                    column: if col > 0 { Some(col) } else { None },
                                    context: Some("Action types must include the namespace, e.g., 'evm::send_eth'".to_string()),
                                    documentation_link: None,
                                });
                                self.blocks_with_errors.insert(name_str.clone());
                            }
                        } else {
                            // Skip validation if this block already has errors from collection phase
                            if self.blocks_with_errors.contains(&name_str) {
                                return;
                            }
                        }
                    }
                }
            }
            "output" => {
                if let Some(BlockLabel::String(name)) = block.labels.get(0) {
                    if let Some(ctx) = &mut self.current_block {
                        ctx.name = name.value().to_string();
                    }

                    if !self.is_validation_phase {
                        self.defined_outputs.insert(name.value().to_string());
                    }
                }
            }
            "variable" => {
                if let Some(BlockLabel::String(name)) = block.labels.get(0) {
                    if let Some(ctx) = &mut self.current_block {
                        ctx.name = name.value().to_string();
                    }

                    if !self.is_validation_phase {
                        self.defined_variables.insert(name.value().to_string());
                    }
                }
            }
            "flow" => {
                if let Some(BlockLabel::String(name)) = block.labels.get(0) {
                    if let Some(ctx) = &mut self.current_block {
                        ctx.name = name.value().to_string();
                    }

                    if !self.is_validation_phase {
                        // Collect flow inputs and location
                        let mut inputs = Vec::new();
                        for attr in block.body.attributes() {
                            if attr.key.as_str() != "description" {
                                inputs.push(attr.key.to_string());
                            }
                        }
                        self.flow_inputs.insert(name.value().to_string(), inputs);

                        // Store flow location for error reporting
                        let (line, col) = self.optional_span_to_position(block.ident.span());
                        self.flow_locations.insert(name.value().to_string(), (line, col));
                    }
                }
            }
            _ => {}
        }
    }

    /// Validate a traversal expression
    fn validate_traversal(&mut self, traversal: &Traversal) {
        // Note: We process traversals in BOTH phases:
        // - Collection phase: to gather input references
        // - Validation phase: to validate references like flow.*, action.*, etc.

        // Get the root variable
        let Some(root) = traversal.expr.as_variable() else {
            return;
        };

        // Build the full path
        let mut parts = VecDeque::new();
        parts.push_back(root.to_string());

        for op in traversal.operators.iter() {
            if let TraversalOperator::GetAttr(attr) = op.value() {
                parts.push_back(attr.to_string());
            }
        }

        if parts.is_empty() {
            return;
        }

        let (line, col) = self
            .current_block
            .as_ref()
            .and_then(|ctx| ctx.span.as_ref())
            .map(|span| self.span_to_position(span))
            .unwrap_or((0, 0));

        // In collection phase, we only collect input/env references
        if !self.is_validation_phase {
            match parts[0].as_str() {
                "input" => {
                    // Collect input reference for later validation
                    let parts_vec: Vec<String> = parts.into_iter().collect();
                    let (line, col) = self
                        .current_block
                        .as_ref()
                        .and_then(|ctx| ctx.span.as_ref())
                        .map(|span| self.span_to_position(span))
                        .unwrap_or((0, 0));

                    self.input_refs.push(LocatedInputRef {
                        name: parts_vec.join("."),
                        line,
                        column: col,
                    });
                }
                "env" => {
                    // Collect environment variable reference for later validation
                    if parts.len() >= 2 {
                        let parts_vec: Vec<String> = parts.into_iter().collect();
                        let (line, col) = self
                            .current_block
                            .as_ref()
                            .and_then(|ctx| ctx.span.as_ref())
                            .map(|span| self.span_to_position(span))
                            .unwrap_or((0, 0));

                        self.input_refs.push(LocatedInputRef {
                            name: parts_vec.join("."),
                            line,
                            column: col,
                        });
                    }
                }
                _ => {}
            }
            return;
        }

        // In validation phase, we validate all references
        match parts[0].as_str() {
            "action" => {
                if parts.len() >= 2 {
                    let action_name = &parts[1];
                    if !self.action_types.contains_key(action_name) {
                        self.result.errors.push(ValidationError {
                            message: format!("Reference to undefined action '{}'", action_name),
                            file: self.file_path.clone(),
                            line: if line > 0 { Some(line) } else { None },
                            column: if col > 0 { Some(col) } else { None },
                            context: Some(
                                "Make sure the action is defined before using it".to_string(),
                            ),
                            documentation_link: None,
                        });
                    } else if parts.len() >= 3 && !self.blocks_with_errors.contains(action_name) {
                        // Validate field access only if action doesn't have errors
                        self.validate_action_field_access(action_name, &parts[2], line, col);
                    }
                }
            }
            "signer" => {
                if parts.len() >= 2 {
                    let signer_name = &parts[1];
                    if !self.defined_signers.contains_key(signer_name) {
                        self.result.errors.push(ValidationError {
                            message: format!("Reference to undefined signer '{}'", signer_name),
                            file: self.file_path.clone(),
                            line: if line > 0 { Some(line) } else { None },
                            column: if col > 0 { Some(col) } else { None },
                            context: Some(
                                "Signers must be defined before they can be referenced".to_string(),
                            ),
                            documentation_link: None,
                        });
                    }
                }
            }
            "variable" => {
                if parts.len() >= 2 {
                    let var_name = &parts[1];
                    if !self.defined_variables.contains(var_name) {
                        self.result.errors.push(ValidationError {
                            message: format!("Reference to undefined variable '{}'", var_name),
                            file: self.file_path.clone(),
                            line: if line > 0 { Some(line) } else { None },
                            column: if col > 0 { Some(col) } else { None },
                            context: Some(
                                "Variables must be defined before they can be referenced"
                                    .to_string(),
                            ),
                            documentation_link: None,
                        });
                    }
                }
            }
            "flow" => {
                if parts.len() >= 2 {
                    let attr_name = &parts[1];

                    if self.flow_inputs.is_empty() {
                        // No flows defined at all
                        self.result.errors.push(ValidationError {
                            message: format!(
                                "Reference to flow.{} but no flows are defined",
                                attr_name
                            ),
                            file: self.file_path.clone(),
                            line: if line > 0 { Some(line) } else { None },
                            column: if col > 0 { Some(col) } else { None },
                            context: Some(
                                "Define at least one flow before referencing flow attributes"
                                    .to_string(),
                            ),
                            documentation_link: None,
                        });
                    } else {
                        // Check which flows are missing this attribute
                        let missing_flows: Vec<String> = self
                            .flow_inputs
                            .iter()
                            .filter(|(_, inputs)| !inputs.contains(&attr_name.to_string()))
                            .map(|(name, _)| name.clone())
                            .collect();

                        if !missing_flows.is_empty() {
                            // First, add an error at the usage site (where flow.attribute is referenced)
                            self.result.errors.push(ValidationError {
                                message: format!(
                                    "Flow attribute '{}' is not defined in {} flow{}: {}",
                                    attr_name,
                                    missing_flows.len(),
                                    if missing_flows.len() > 1 { "s" } else { "" },
                                    missing_flows.join(", ")
                                ),
                                file: self.file_path.clone(),
                                line: if line > 0 { Some(line) } else { None },
                                column: if col > 0 { Some(col) } else { None },
                                context: Some(
                                    "This attribute must be defined in all flows".to_string(),
                                ),
                                documentation_link: None,
                            });

                            // Then, create an error for each flow that's missing the attribute
                            for flow_name in &missing_flows {
                                let (flow_line, flow_col) =
                                    self.flow_locations.get(flow_name).copied().unwrap_or((0, 0));

                                self.result.errors.push(ValidationError {
                                    message: format!(
                                        "Flow '{}' is missing attribute '{}' which is required by actions", 
                                        flow_name,
                                        attr_name
                                    ),
                                    file: self.file_path.clone(),
                                    line: if flow_line > 0 { Some(flow_line) } else { None },
                                    column: if flow_col > 0 { Some(flow_col) } else { None },
                                    context: Some(format!(
                                        "Add '{} = <value>' to this flow since it's referenced at line {}", 
                                        attr_name,
                                        line
                                    )),
                                    documentation_link: None,
                                });
                            }
                        }
                    }
                }
            }
            "output" => {
                // Output references need ordering validation
                if parts.len() >= 2 {
                    let output_name = &parts[1];
                    // In output context, check for circular dependencies
                    if let Some(ctx) = &self.current_block {
                        if ctx.block_type == "output" && !self.defined_outputs.contains(output_name)
                        {
                            self.result.errors.push(ValidationError {
                                message: format!(
                                    "Output '{}' references undefined output '{}'",
                                    ctx.name, output_name
                                ),
                                file: self.file_path.clone(),
                                line: if line > 0 { Some(line) } else { None },
                                column: if col > 0 { Some(col) } else { None },
                                context: Some(
                                    "Outputs can only reference previously defined outputs"
                                        .to_string(),
                                ),
                                documentation_link: None,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Validate action field access
    fn validate_action_field_access(
        &mut self,
        action_name: &str,
        field: &str,
        line: usize,
        col: usize,
    ) {
        if let Some(spec) = self.action_specs.get(action_name) {
            let output_names: Vec<String> = spec.outputs.iter().map(|o| o.name.clone()).collect();

            if !output_names.contains(&field.to_string()) {
                let action_type = self.action_types.get(action_name).unwrap();
                self.result.errors.push(ValidationError {
                    message: format!(
                        "Field '{}' does not exist on action '{}' ({}). Available outputs: {}",
                        field,
                        action_name,
                        action_type,
                        output_names.join(", ")
                    ),
                    file: self.file_path.clone(),
                    line: if line > 0 { Some(line) } else { None },
                    column: if col > 0 { Some(col) } else { None },
                    context: None,
                    documentation_link: action_type
                        .split_once("::")
                        .and_then(|(ns, action)| get_action_doc_link(ns, action)),
                });
            }
        }
    }

    /// Run two-pass validation on the body
    pub fn validate(&mut self, body: &Body) {
        // Pass 1: Collection phase
        self.is_validation_phase = false;
        self.visit_body(body);

        // Pass 2: Validation phase
        self.is_validation_phase = true;
        self.visit_body(body);

        // Merge primary errors (namespace/type errors) with other errors
        // Primary errors go first as they're more fundamental
        let mut all_errors = std::mem::take(&mut self.primary_errors);
        all_errors.append(&mut self.result.errors);
        self.result.errors = all_errors;
    }
}

impl<'a> Visit for HclValidationVisitor<'a> {
    fn visit_block(&mut self, block: &Block) {
        self.process_block(block);

        // During validation phase, skip blocks that had errors in collection phase
        if self.is_validation_phase {
            if let Some(ctx) = &self.current_block {
                if self.blocks_with_errors.contains(&ctx.name) {
                    // Skip validation of this block's contents
                    return;
                }
            }
        }

        // Continue visiting the block's contents
        visit_block(self, block);
    }

    fn visit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Traversal(traversal) => {
                self.validate_traversal(traversal);
            }
            _ => {}
        }

        // Continue visiting nested expressions
        visit_expr(self, expr);
    }
}

/// Run HCL-based validation on a runbook with custom addon specifications
pub fn validate_with_hcl_and_addons(
    content: &str,
    result: &mut ValidationResult,
    file_path: &str,
    addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
) -> Result<Vec<LocatedInputRef>, String> {
    // Parse the content as HCL
    let body: Body = content.parse().map_err(|e| format!("Failed to parse runbook: {}", e))?;

    // Create and run the validator with custom addon specs
    let mut visitor =
        HclValidationVisitor::new_with_addons(result, file_path, content, addon_specs);
    visitor.validate(&body);

    Ok(visitor.input_refs)
}

/// Run HCL-based validation on a runbook (uses default addon specifications)
pub fn validate_with_hcl(
    content: &str,
    result: &mut ValidationResult,
    file_path: &str,
) -> Result<Vec<LocatedInputRef>, String> {
    // Parse the content as HCL
    let body: Body = content.parse().map_err(|e| format!("Failed to parse runbook: {}", e))?;

    // Create and run the validator
    let mut visitor = HclValidationVisitor::new(result, file_path, content);
    visitor.validate(&body);

    Ok(visitor.input_refs)
}
