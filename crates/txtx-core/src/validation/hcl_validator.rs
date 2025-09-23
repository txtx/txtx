//! HCL-based validation for the doctor command using hcl-edit
//!
//! This module uses hcl-edit's visitor pattern to perform comprehensive
//! validation of runbook files, replacing the Tree-sitter based approach.

use std::collections::{HashMap, HashSet, VecDeque};

use txtx_addon_kit::hcl::{
    expr::{Expression, Traversal, TraversalOperator},
    structure::{Attribute, Block, BlockLabel, Body},
    visit::{visit_attr, visit_block, visit_expr, Visit},
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

/// A basic HCL validator that performs structural validation without addon specifications.
/// This validator can check HCL syntax, references between blocks, and structural correctness,
/// but cannot validate action parameters since it lacks addon specifications.
pub struct BasicHclValidator<'a> {
    inner: HclValidationVisitor<'a>,
}

/// A full HCL validator with addon specifications for comprehensive validation.
/// This validator can perform all validations including action parameter checking.
pub struct FullHclValidator<'a> {
    inner: HclValidationVisitor<'a>,
}

impl<'a> BasicHclValidator<'a> {
    /// Create a basic validator for structural validation only.
    /// This validator cannot check action parameters since it lacks addon specifications.
    pub fn new(result: &'a mut ValidationResult, file_path: &str, source: &'a str) -> Self {
        Self {
            inner: HclValidationVisitor::new_with_addons(result, file_path, source, HashMap::new()),
        }
    }

    /// Run validation on the HCL body
    pub fn validate(&mut self, body: &Body) -> Vec<LocatedInputRef> {
        self.inner.validate(body);
        std::mem::take(&mut self.inner.input_refs)
    }
}

impl<'a> FullHclValidator<'a> {
    /// Create a full validator with addon specifications for comprehensive validation.
    /// This validator can check all aspects including action parameters.
    pub fn new(
        result: &'a mut ValidationResult,
        file_path: &str,
        source: &'a str,
        addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        Self {
            inner: HclValidationVisitor::new_with_addons(result, file_path, source, addon_specs),
        }
    }

    /// Run validation on the HCL body
    pub fn validate(&mut self, body: &Body) -> Vec<LocatedInputRef> {
        self.inner.validate(body);
        std::mem::take(&mut self.inner.input_refs)
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
    
    // === Action Validation ===
    /// Track attributes seen in current action block
    seen_action_attributes: HashSet<String>,
}

#[derive(Clone, Debug)]
struct BlockContext {
    block_type: String,
    name: String,
    span: Option<std::ops::Range<usize>>,
}

impl<'a> HclValidationVisitor<'a> {
    /// Internal constructor with addon specifications
    fn new_with_addons(
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
            seen_action_attributes: HashSet::new(),
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
                            // Clear seen attributes for this new action block
                            self.seen_action_attributes.clear();
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

    /// Check for missing required parameters in the current action block
    fn check_missing_required_parameters(&mut self) {
        // Early return if not in validation phase
        if !self.is_validation_phase {
            return;
        }

        let ctx = match &self.current_block {
            Some(ctx) if ctx.block_type == "action" => ctx,
            _ => return,
        };

        let spec = match self.action_specs.get(&ctx.name) {
            Some(spec) => spec,
            None => return,
        };

        let action_type = match self.action_types.get(&ctx.name) {
            Some(action_type) => action_type,
            None => return, // Defensive: shouldn't happen if spec exists
        };

        // Get position from block context if available
        // Use (0, 0) to indicate unknown position rather than misleading (1, 1)
        let (line, col) = ctx.span.as_ref()
            .map(|span| self.span_to_position(span))
            .unwrap_or((0, 0));

        // Check each required input
        for input in &spec.inputs {
            if input.optional || input.internal || self.seen_action_attributes.contains(&input.name) {
                continue;
            }

            self.result.errors.push(MissingParameterError {
                parameter_name: &input.name,
                action_name: &ctx.name,
                action_type,
                documentation: &input.documentation,
                file_path: &self.file_path,
                line,
                column: col,
            }.into());
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

    /// Validate an attribute in an action block
    fn validate_action_attribute(&mut self, attr: &Attribute, ctx: &BlockContext, spec: &CommandSpecification) {
        let attr_name = attr.key.as_str();

        // Track that we've seen this attribute
        self.seen_action_attributes.insert(attr_name.to_string());

        // Collect valid input names
        let valid_inputs: HashSet<String> = spec.inputs
            .iter()
            .map(|input| input.name.clone())
            .collect();

        // Check if this attribute is a valid input
        if valid_inputs.contains(attr_name) || spec.accepts_arbitrary_inputs {
            return; // Valid attribute
        }

        // Get action type (defensive: should always exist if spec exists)
        let action_type = match self.action_types.get(&ctx.name) {
            Some(action_type) => action_type,
            None => return,
        };

        // Get position
        let (line, col) = self.optional_span_to_position(attr.span());

        // Create list of available parameters for error message
        let available_inputs: Vec<String> = spec.inputs
            .iter()
            .filter(|input| !input.internal)
            .map(|input| {
                if input.optional {
                    format!("{} (optional)", input.name)
                } else {
                    input.name.clone()
                }
            })
            .collect();

        self.result.errors.push(InvalidParameterError {
            parameter_name: attr_name,
            action_name: &ctx.name,
            action_type,
            available_inputs,
            file_path: &self.file_path,
            line,
            column: col,
        }.into());
    }
}

/// Helper struct for creating missing parameter errors
struct MissingParameterError<'a> {
    parameter_name: &'a str,
    action_name: &'a str,
    action_type: &'a str,
    documentation: &'a str,
    file_path: &'a str,
    line: usize,
    column: usize,
}

impl<'a> From<MissingParameterError<'a>> for ValidationError {
    fn from(error: MissingParameterError<'a>) -> Self {
        let (line, column) = if error.line > 0 { 
            (Some(error.line), Some(error.column)) 
        } else { 
            (None, None) 
        };
        
        ValidationError {
            message: format!(
                "Missing required parameter '{}' for action '{}' ({})",
                error.parameter_name,
                error.action_name,
                error.action_type
            ),
            file: error.file_path.to_string(),
            line,
            column,
            context: Some(format!("{}: {}", error.parameter_name, error.documentation)),
            documentation_link: error.action_type
                .split_once("::")
                .and_then(|(ns, action)| get_action_doc_link(ns, action)),
        }
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
        
        // After visiting all attributes, check for missing required parameters
        // This must be done AFTER visit_block to ensure all attributes have been seen
        // Note: check_missing_required_parameters already checks is_validation_phase internally
        if matches!(&self.current_block, Some(ctx) if ctx.block_type == "action") {
            self.check_missing_required_parameters();
        }
    }

    fn visit_attr(&mut self, attr: &Attribute) {
        // Early return if not in validation phase
        if !self.is_validation_phase {
            visit_attr(self, attr);
            return;
        }

        // Check if we're in an action block that needs validation
        let ctx = match &self.current_block {
            Some(ctx) if ctx.block_type == "action" && !self.blocks_with_errors.contains(&ctx.name) => ctx.clone(),
            _ => {
                visit_attr(self, attr);
                return;
            }
        };

        // Get the action specification
        let spec = match self.action_specs.get(&ctx.name) {
            Some(spec) => spec.clone(),
            None => {
                visit_attr(self, attr);
                return;
            }
        };

        // Validate the attribute
        self.validate_action_attribute(attr, &ctx, &spec);

        // Continue visiting the attribute's value
        visit_attr(self, attr);
    }

    fn visit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Traversal(traversal) => {
                self.validate_traversal(traversal);
            }
            _ => {}
        }
        visit_expr(self, expr);
    }
}

/// Helper struct for creating invalid parameter errors
struct InvalidParameterError<'a> {
    parameter_name: &'a str,
    action_name: &'a str,
    action_type: &'a str,
    available_inputs: Vec<String>,
    file_path: &'a str,
    line: usize,
    column: usize,
}

impl<'a> From<InvalidParameterError<'a>> for ValidationError {
    fn from(error: InvalidParameterError<'a>) -> Self {
        let (line, column) = if error.line > 0 { 
            (Some(error.line), Some(error.column)) 
        } else { 
            (None, None) 
        };
        
        let available_params = if error.available_inputs.is_empty() {
            "none".to_string()
        } else {
            error.available_inputs.join(", ")
        };
        
        ValidationError {
            message: format!(
                "Invalid parameter '{}' for action '{}' ({}). Available parameters: {}",
                error.parameter_name,
                error.action_name,
                error.action_type,
                available_params
            ),
            file: error.file_path.to_string(),
            line,
            column,
            context: error.action_type
                .split_once("::")
                .and_then(|(ns, action)| get_action_doc_link(ns, action))
                .map(|link| format!("See documentation: {}", link)),
            documentation_link: None,
        }
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

    // Create and run the full validator with addon specs
    let mut validator = FullHclValidator::new(result, file_path, content, addon_specs);
    let input_refs = validator.validate(&body);

    Ok(input_refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kit::types::commands::{CommandInput, CommandOutput};
    use crate::kit::types::types::Type;

    // TODO: Fix these tests once CommandSpecification structure is stabilized
    // The tests are temporarily disabled due to struct field mismatches
    /*
    #[test]
    fn test_position_handling_with_missing_span() {
        // Test that when a block has no span (None), the validation error
        // correctly gets position (0, 0) which converts to None in the error
        let mut result = ValidationResult::default();
        let content = "test content";
        let file_path = "test.tx";

        // Create a visitor with addon specs that has an action spec
        let mut addon_specs = HashMap::new();
        let mut action_spec = CommandSpecification {
            name: "test_action".to_string(),
            matcher: "test_action".to_string(),
            documentation: "Test action".to_string(),
            implements_signing_capability: false,
            inputs: vec![
                crate::kit::types::commands::CommandInput {
                    name: "required_param".to_string(),
                    documentation: "A required parameter".to_string(),
                    typing: crate::kit::types::types::Type::string(),
                    optional: false,
                    internal: false,
                    tainting: false,
                }
            ],
            outputs: vec![],
            default_inputs: HashMap::new(),
            example: None,
        };
        addon_specs.insert("test".to_string(), vec![("test_action".to_string(), action_spec)]);

        let mut visitor = HclValidationVisitor::new_with_addons(&mut result, file_path, content, addon_specs);

        // Manually set up visitor state to simulate being in validation phase
        // with an action block that has no span
        visitor.is_validation_phase = true;
        visitor.current_block = Some(BlockContext {
            block_type: "action".to_string(),
            name: "my_action".to_string(),
            span: None, // This is the key - no span information
        });
        visitor.action_types.insert("my_action".to_string(), "test::test_action".to_string());
        visitor.action_specs.insert("my_action".to_string(),
            visitor.addon_specs.get("test").unwrap()[0].1.clone());

        // Call the actual method that would use unwrap_or((0, 0))
        visitor.check_missing_required_parameters();

        // Check that an error was created with None for position
        assert!(!visitor.result.errors.is_empty(), "Should have error for missing required parameter");

        let error = &visitor.result.errors[0];
        assert!(error.message.contains("Missing required parameter"));

        // When span is None, position should be None (not Some(0) or Some(1))
        assert_eq!(error.line, None, "Line should be None when span is missing");
        assert_eq!(error.column, None, "Column should be None when span is missing");
    }

    #[test]
    fn test_position_handling_with_valid_span() {
        // Test that when a block has a valid span, the validation error
        // gets the correct line/column position
        let mut result = ValidationResult::default();
        let content = "# First line\naction \"test\" \"test::test_action\" {\n  # Third line\n}";
        let file_path = "test.tx";

        // Create a visitor with addon specs
        let mut addon_specs = HashMap::new();
        let mut action_spec = CommandSpecification::default();
        action_spec.name = "test_action".to_string();
        action_spec.inputs = vec![
            crate::kit::types::commands::CommandInput {
                name: "required_param".to_string(),
                optional: false,
                internal: false,
                ..Default::default()
            }
        ];
        addon_specs.insert("test".to_string(), vec![("test_action".to_string(), action_spec)]);

        let mut visitor = HclValidationVisitor::new_with_addons(&mut result, file_path, content, addon_specs);

        // Set up visitor state with a valid span
        visitor.is_validation_phase = true;
        visitor.current_block = Some(BlockContext {
            block_type: "action".to_string(),
            name: "test".to_string(),
            span: Some(13..49), // Points to the action block on line 2
        });
        visitor.action_types.insert("test".to_string(), "test::test_action".to_string());
        visitor.action_specs.insert("test".to_string(),
            visitor.addon_specs.get("test").unwrap()[0].1.clone());

        // Call the method that creates validation errors
        visitor.check_missing_required_parameters();

        // Check that error has correct position
        assert!(!visitor.result.errors.is_empty(), "Should have error for missing required parameter");

        let error = &visitor.result.errors[0];
        assert!(error.line.is_some(), "Line should be Some when span exists");
        assert!(error.column.is_some(), "Column should be Some when span exists");

        // The position should be line 2 (the action block starts on line 2)
        assert_eq!(error.line.unwrap(), 2, "Error should be on line 2");
        assert!(error.column.unwrap() > 0, "Column should be > 0");
    }
    */

    // TODO: Fix these tests once CommandSpecification structure is finalized
    // The tests below are temporarily disabled due to struct field mismatches
    /*
    #[test]
    fn test_position_fallback_to_zero() {
        let mut result = ValidationResult::new();
        let source = "action \"test\" \"evm::send_eth\" {}";

        // Create a minimal addon spec for send_eth
        let mut addon_specs = HashMap::new();
        let send_eth_spec = CommandSpecification {
            name: "send_eth".to_string(),
            namespace: "evm".to_string(),
            implements: vec![],
            description: Some("Send ETH".to_string()),
            documentation_url: None,
            inputs: vec![
                CommandInput {
                    name: "signer".to_string(),
                    description: Some("Signer".to_string()),
                    documentation_url: None,
                    type_bounds: vec![],
                    required: true,
                    default_value: None,
                    depends_on: vec![],
                },
                CommandInput {
                    name: "recipient_address".to_string(),
                    description: Some("Recipient".to_string()),
                    documentation_url: None,
                    type_bounds: vec![],
                    required: true,
                    default_value: None,
                    depends_on: vec![],
                },
            ],
            outputs: vec![CommandOutput {
                name: "tx_hash".to_string(),
                description: Some("Transaction hash".to_string()),
                documentation_url: None,
                r#type: Type::String(crate::kit::types::types::StringType {
                    default: None,
                    min_length: None,
                    max_length: None,
                    regex: None,
                }),
                depends_on: vec![],
            }],
            broadcast: None,
            evaluator: None,
            available_filters: vec![],
        };

        addon_specs.insert("evm".to_string(), vec![("send_eth".to_string(), send_eth_spec)]);

        let mut visitor = HclValidationVisitor::new_with_addons(
            &mut result,
            "test.tx",
            source,
            addon_specs,
        );

        // Simulate a block with Some span - normal case
        visitor.current_block = Some(BlockContext {
            block_name: "test".to_string(),
            block_type: "action".to_string(),
            action_type: Some("evm::send_eth".to_string()),
            span: Some(Span::new(0, 33)),  // Has a span
            visited_parameters: HashSet::new(),
        });

        visitor.check_missing_required_parameters();

        // Should have errors for missing required parameters
        assert!(result.errors.len() >= 2, "Should have errors for missing signer and recipient_address");

        // Check that errors have proper span information
        for error in &result.errors {
            assert!(error.span.is_some(), "Errors should have span when block has span");
        }
    }

    #[test]
    fn test_position_when_span_is_none() {
        let mut result = ValidationResult::new();
        let source = "action \"test\" \"evm::send_eth\" { signer = signer.alice }";

        let mut addon_specs = HashMap::new();
        let send_eth_spec = CommandSpecification {
            name: "send_eth".to_string(),
            namespace: "evm".to_string(),
            implements: vec![],
            description: Some("Send ETH".to_string()),
            documentation_url: None,
            inputs: vec![
                CommandInput {
                    name: "signer".to_string(),
                    description: Some("Signer".to_string()),
                    documentation_url: None,
                    type_bounds: vec![],
                    required: true,
                    default_value: None,
                    depends_on: vec![],
                },
                CommandInput {
                    name: "recipient_address".to_string(),
                    description: Some("Recipient".to_string()),
                    documentation_url: None,
                    type_bounds: vec![],
                    required: true,
                    default_value: None,
                    depends_on: vec![],
                },
            ],
            outputs: vec![CommandOutput {
                name: "tx_hash".to_string(),
                description: Some("Transaction hash".to_string()),
                documentation_url: None,
                r#type: Type::String(crate::kit::types::types::StringType {
                    default: None,
                    min_length: None,
                    max_length: None,
                    regex: None,
                }),
                depends_on: vec![],
            }],
            broadcast: None,
            evaluator: None,
            available_filters: vec![],
        };

        addon_specs.insert("evm".to_string(), vec![("send_eth".to_string(), send_eth_spec)]);

        let mut visitor = HclValidationVisitor::new_with_addons(
            &mut result,
            "test.tx",
            source,
            addon_specs,
        );

        // Simulate visiting a block with no span
        visitor.current_block = Some(BlockContext {
            block_name: "test".to_string(),
            block_type: "action".to_string(),
            action_type: Some("evm::send_eth".to_string()),
            span: None,  // No span, so position should be (0, 0)
            visited_parameters: HashSet::new(),
        });

        // Call the method that checks for missing parameters
        visitor.check_missing_required_parameters();

        // Should have an error for missing recipient_address
        assert_eq!(result.errors.len(), 1);
        let error = &result.errors[0];
        assert!(error.message.contains("Missing required parameter 'recipient_address'"));

        // When span is None, position should be None (converted from (0, 0))
        assert!(error.span.is_none(), "Expected span to be None when block has no span");
    }
    */
}

/// Run HCL-based validation on a runbook (limited validation without addon specs)
pub fn validate_with_hcl(
    content: &str,
    result: &mut ValidationResult,
    file_path: &str,
) -> Result<Vec<LocatedInputRef>, String> {
    // Parse the content as HCL
    let body: Body = content.parse().map_err(|e| format!("Failed to parse runbook: {}", e))?;

    // Create and run the basic validator (structural validation only)
    // Note: This validator cannot validate action parameters since addon specs are not available
    let mut validator = BasicHclValidator::new(result, file_path, content);
    let input_refs = validator.validate(&body);

    Ok(input_refs)
}
