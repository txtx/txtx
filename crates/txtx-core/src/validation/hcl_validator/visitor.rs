//! HCL validation visitor for txtx runbooks.
//!
//! This module provides two-phase validation of HCL runbooks:
//!
//! 1. **Collection phase**: Gathers all definitions (variables, signers, actions, flows)
//! 2. **Validation phase**: Validates references and checks for circular dependencies
//!
//! # Examples
//!
//! ```no_run
//! use txtx_core::validation::hcl_validator::{BasicHclValidator, validate_with_hcl};
//! use txtx_core::validation::types::ValidationResult;
//!
//! let mut result = ValidationResult::new();
//! let content = "variable \"foo\" { default = \"bar\" }";
//! let refs = validate_with_hcl(content, &mut result, "main.tx").unwrap();
//! ```

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use txtx_addon_kit::hcl::{
    expr::{Expression, Traversal, TraversalOperator},
    structure::{Block, BlockLabel, Body},
    visit::{visit_block, visit_expr, Visit},
    Span,
};
use txtx_addon_kit::constants::{
    DESCRIPTION, DEPENDS_ON, MARKDOWN, MARKDOWN_FILEPATH, POST_CONDITION, PRE_CONDITION,
};

use crate::runbook::location::{SourceMapper, BlockContext};
use crate::validation::types::{LocatedInputRef, ValidationError as LegacyError, ValidationResult};
use crate::kit::types::commands::CommandSpecification;

use super::dependency_graph::DependencyGraph;
use super::block_processors;

/// Validation errors.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Missing required label: {0}")]
    MissingLabel(&'static str),

    #[error("Invalid format: {value}. Expected: {expected}")]
    InvalidFormat { value: String, expected: &'static str },

    #[error("Unknown namespace: {namespace}. Available: {}", available.join(", "))]
    UnknownNamespace {
        namespace: String,
        available: Vec<String>,
    },

    #[error("Unknown action: {namespace}::{action}")]
    UnknownAction {
        namespace: String,
        action: String,
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Undefined {construct_type}: '{name}'")]
    UndefinedReference {
        construct_type: String,
        name: String,
    },

    #[error("Missing parameter '{param}' for action '{action}'")]
    MissingParameter { param: String, action: String },

    #[error("Invalid parameter '{param}' for action '{action}'")]
    InvalidParameter { param: String, action: String },

    #[error("Output field '{field}' does not exist for action '{action_name}'. Available fields: {}", available.join(", "))]
    InvalidOutputField {
        action_name: String,
        field: String,
        available: Vec<String>,
    },

    #[error("circular dependency in {construct_type}: {}", cycle.join(" -> "))]
    CircularDependency {
        construct_type: String,
        cycle: Vec<String>,
    },
}

/// Block types in HCL runbooks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Action,
    Signer,
    Variable,
    Output,
    Flow,
    Secret,
    Addon,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EntityType {
    Variable,
    Action,
}

impl BlockType {
    fn from_str(s: &str) -> Self {
        match s {
            "action" => Self::Action,
            "signer" => Self::Signer,
            "variable" => Self::Variable,
            "output" => Self::Output,
            "flow" => Self::Flow,
            "secret" => Self::Secret,
            "addon" => Self::Addon,
            _ => Self::Unknown,
        }
    }
}

/// Items collected during the collection phase.

#[derive(Debug)]
pub enum CollectedItem {
    Definition(DefinitionItem),
    Declaration(DeclarationItem),
    Dependencies {
        entity_type: String,
        entity_name: String,
        depends_on: Vec<String>,
    },
}

#[derive(Debug)]
pub enum DefinitionItem {
    Variable { name: String, position: Position },
    Signer { name: String, signer_type: String },
    Output(String),
    Secret(String),
}

#[derive(Debug)]
pub enum DeclarationItem {
    Action {
        name: String,
        action_type: String,
        spec: Option<CommandSpecification>,
        position: Position,
    },
    Flow {
        name: String,
        inputs: Vec<String>,
        position: Position,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

#[derive(Debug, Clone)]
struct FlowInputReference {
    input_name: String,
    location: Position,
    file_path: String,
    context: BlockContext,
}

#[derive(Debug, Clone, Copy)]
enum DependencyType {
    Variable,
    Action,
}


mod validation_rules {
    use super::*;

    /// Validate action format (namespace::action)
    pub fn validate_action_format(action: &str) -> Result<(&str, &str), ValidationError> {
        action
            .split_once("::")
            .ok_or_else(|| ValidationError::InvalidFormat {
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
        namespace_actions
            .iter()
            .find(|(matcher, _)| matcher == action)
            .map(|(_, spec)| spec)
    }

    /// Validate a complete action
    pub fn validate_action(
        action_type: &str,
        specs: &HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Result<CommandSpecification, ValidationError> {
        let (namespace, action) = validate_action_format(action_type)?;
        let namespace_actions = validate_namespace_exists(namespace, specs)?;

        find_action_spec(action, namespace_actions)
            .cloned()
            .ok_or_else(|| ValidationError::UnknownAction {
                namespace: namespace.to_string(),
                action: action.to_string(),
                cause: None,
            })
    }

    /// Check if an attribute is an inherited property
    pub fn is_inherited_property(attr_name: &str) -> bool {
        matches!(
            attr_name,
            MARKDOWN | MARKDOWN_FILEPATH | DESCRIPTION | DEPENDS_ON | PRE_CONDITION | POST_CONDITION
        )
    }
}



/// Helper to convert SourceMapper results to Position (without file)
fn source_mapper_to_position(mapper: &SourceMapper, span: &std::ops::Range<usize>) -> Position {
    let (line, col) = mapper.span_to_position(span);
    Position::new(line, col)
}

fn optional_span_to_position(mapper: &SourceMapper, span: Option<&std::ops::Range<usize>>) -> Position {
    span.map(|s| source_mapper_to_position(mapper, s))
        .unwrap_or_default()
}




#[derive(Default)]
struct ValidationState {
    definitions: Definitions,
    declarations: Declarations,
    dependency_graphs: DependencyGraphs,
    input_refs: Vec<LocatedInputRef>,
    flow_input_refs: HashMap<String, Vec<FlowInputReference>>,
}

#[derive(Default)]
struct Definitions {
    variables: HashSet<String>,
    signers: HashMap<String, String>,
    outputs: HashSet<String>,
}

#[derive(Default)]
struct Declarations {
    variables: HashMap<String, VariableDeclaration>,
    actions: HashMap<String, ActionDeclaration>,
    flows: HashMap<String, FlowDeclaration>,
}

struct VariableDeclaration {
    position: Position,
}

struct ActionDeclaration {
    action_type: String,
    spec: Option<CommandSpecification>,
    position: Position,
}

struct FlowDeclaration {
    inputs: Vec<String>,
    position: Position,
}

#[derive(Default)]
struct DependencyGraphs {
    variables: DependencyGraph,
    actions: DependencyGraph,
}

impl ValidationState {
    /// Apply collected items using iterator chains
    fn apply_items(&mut self, items: Vec<CollectedItem>) {
        use CollectedItem::*;
        use DefinitionItem::*;
        use DeclarationItem::*;

        items.into_iter().for_each(|item| match item {
            Definition(def) => match def {
                Variable { name, position } => {
                    self.definitions.variables.insert(name.clone());
                    self.dependency_graphs.variables.add_node(name.clone(), None);
                    self.declarations.variables.insert(name, VariableDeclaration { position });
                }
                Signer { name, signer_type } => {
                    self.definitions.signers.insert(name, signer_type);
                }
                Output(name) => {
                    self.definitions.outputs.insert(name);
                }
                Secret(name) => {
                    self.definitions.variables.insert(name);
                }
            },
            Declaration(decl) => match decl {
                Action { name, action_type, spec, position } => {
                    self.declarations.actions.insert(name.clone(), ActionDeclaration {
                        action_type,
                        spec,
                        position,
                    });
                    self.dependency_graphs.actions.add_node(name, None);
                }
                Flow { name, inputs, position } => {
                    self.declarations.flows.insert(name, FlowDeclaration {
                        inputs,
                        position,
                    });
                }
            },
            Dependencies { entity_type, entity_name, depends_on } => {
                // Add dependency edges using iterator and match
                if let Some(graph) = match entity_type.as_str() {
                    "variable" => Some(&mut self.dependency_graphs.variables),
                    "action" => Some(&mut self.dependency_graphs.actions),
                    _ => None,
                } {
                    depends_on.into_iter()
                        .for_each(|dep| graph.add_edge(&entity_name, dep))
                }
            }
        })
    }
}


struct ValidationPhaseHandler<'a> {
    state: &'a ValidationState,
    source_mapper: &'a SourceMapper<'a>,
    file_path: &'a str,
}

impl<'a> ValidationPhaseHandler<'a> {
    fn validate_reference(&self, parts: &[String], position: Position) -> Result<(), ValidationError> {
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0].as_str() {
            "var" | "variable" => self.validate_variable_reference(parts, position),
            "action" => self.validate_action_reference(parts, position),
            "signer" => self.validate_signer_reference(parts, position),
            "output" => self.validate_output_reference(parts, position),
            "flow" => self.validate_flow_reference(parts, position),
            _ => Ok(()),
        }
    }

    fn validate_variable_reference(&self, parts: &[String], _position: Position) -> Result<(), ValidationError> {
        if parts.len() < 2 {
            return Ok(());
        }

        let name = &parts[1];
        if !self.state.definitions.variables.contains(name) {
            return Err(ValidationError::UndefinedReference {
                construct_type: "variable".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    fn validate_action_reference(&self, parts: &[String], _position: Position) -> Result<(), ValidationError> {
        match parts.get(1) {
            None => Ok(()),
            Some(name) => {
                // Check if action exists and get its declaration
                let action = self.state.declarations.actions.get(name)
                    .ok_or_else(|| ValidationError::UndefinedReference {
                        construct_type: "action".to_string(),
                        name: name.to_string(),
                    })?;

                // Validate output field if present
                match (parts.get(2), &action.spec) {
                    (Some(field_name), Some(spec)) => {
                        let valid_outputs: Vec<String> = spec.outputs.iter()
                            .map(|output| output.name.clone())
                            .collect();

                        spec.outputs.iter()
                            .any(|output| &output.name == field_name)
                            .then_some(())
                            .ok_or_else(|| ValidationError::InvalidOutputField {
                                action_name: name.to_string(),
                                field: field_name.to_string(),
                                available: valid_outputs,
                            })
                    }
                    _ => Ok(()),
                }
            }
        }
    }

    fn validate_signer_reference(&self, parts: &[String], _position: Position) -> Result<(), ValidationError> {
        if parts.len() < 2 {
            return Ok(());
        }

        let name = &parts[1];
        if !self.state.definitions.signers.contains_key(name) {
            return Err(ValidationError::UndefinedReference {
                construct_type: "signer".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    fn validate_output_reference(&self, parts: &[String], _position: Position) -> Result<(), ValidationError> {
        if parts.len() < 2 {
            return Ok(());
        }

        let name = &parts[1];
        if !self.state.definitions.outputs.contains(name) {
            return Err(ValidationError::UndefinedReference {
                construct_type: "output".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    fn validate_flow_reference(&self, parts: &[String], _position: Position) -> Result<(), ValidationError> {
        // Flow inputs are now tracked and validated after the collection phase
        // This method is kept for compatibility but doesn't perform immediate validation
        match parts.get(1) {
            None => Ok(()),
            Some(_attr_name) => {
                // Defer validation to the flow validation phase
                Ok(())
            }
        }
    }
}

/// Main HCL validation visitor.

pub struct HclValidationVisitor<'a> {
    result: &'a mut ValidationResult,
    file_path: Cow<'a, str>,
    source_mapper: SourceMapper<'a>,
    addon_specs: &'a HashMap<String, Vec<(String, CommandSpecification)>>,
    state: ValidationState,
}

impl<'a> HclValidationVisitor<'a> {
    pub fn new(
        result: &'a mut ValidationResult,
        file_path: &'a str,
        source: &'a str,
        addon_specs: &'a HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        Self {
            result,
            file_path: Cow::Borrowed(file_path),
            source_mapper: SourceMapper::new(source),
            addon_specs,
            state: ValidationState::default(),
        }
    }

    pub fn validate(&mut self, body: &Body) -> Vec<LocatedInputRef> {
        // Phase 1: Collection (functional approach)
        self.collect_definitions(body);

        // Check cycles
        self.check_circular_dependencies();

        // Validate action types are known
        self.validate_action_types();

        // Phase 2: Validation
        self.validate_references(body);

        // Validate flow inputs after references are collected
        self.validate_all_flow_inputs();

        std::mem::take(&mut self.state.input_refs)
    }

    fn collect_definitions(&mut self, body: &Body) {
        // Collect all blocks using iterator chains
        let items: Vec<CollectedItem> = body.blocks()
            .filter_map(|block| {
                let block_type = BlockType::from_str(block.ident.value());
                block_processors::process_block(block, block_type, self.addon_specs, &self.source_mapper).ok()
            })
            .flatten()
            .collect();

        self.state.apply_items(items);
    }

    fn check_circular_dependencies(&mut self) {
        // Check for cycles using functional approach - report ALL cycles
        self.state.dependency_graphs.variables.find_all_cycles()
            .into_iter()
            .for_each(|cycle| self.report_cycle_error(DependencyType::Variable, cycle));

        self.state.dependency_graphs.actions.find_all_cycles()
            .into_iter()
            .for_each(|cycle| self.report_cycle_error(DependencyType::Action, cycle));
    }

    fn report_cycle_error(&mut self, dependency_type: DependencyType, cycle: Vec<String>) {
        // Get positions for all items in the cycle (excluding the duplicate last element)
        let cycle_len = cycle.len();
        let unique_cycle_items = if cycle_len > 0 && cycle.first() == cycle.last() {
            &cycle[..cycle_len - 1]  // Exclude the duplicate last element
        } else {
            &cycle[..]
        };

        let positions: Vec<Position> = unique_cycle_items
            .iter()
            .filter_map(|name| self.get_declaration_position(&dependency_type, name))
            .collect();

        // Report at first and last positions in the cycle
        match (positions.first(), positions.last()) {
            (Some(&first_pos), Some(&last_pos)) => {
                let construct_type = match dependency_type {
                    DependencyType::Variable => "variable",
                    DependencyType::Action => "action",
                };

                // Always report at the first position
                let error = ValidationError::CircularDependency {
                    construct_type: construct_type.to_string(),
                    cycle: cycle.clone(),
                };
                self.add_error(error, first_pos);

                // Only report at last position if it's different from first
                if first_pos.line != last_pos.line || first_pos.column != last_pos.column {
                    let error = ValidationError::CircularDependency {
                        construct_type: construct_type.to_string(),
                        cycle,
                    };
                    self.add_error(error, last_pos);
                }
            }
            _ => {
                // Fallback when we can't determine positions
                let construct_type = match dependency_type {
                    DependencyType::Variable => "variable",
                    DependencyType::Action => "action",
                };

                let error = ValidationError::CircularDependency {
                    construct_type: construct_type.to_string(),
                    cycle,
                };
                // Report at a default position rather than silently failing
                self.add_error(error, Position::default());
            }
        }
    }

    fn get_declaration_position(&self, dependency_type: &DependencyType, name: &str) -> Option<Position> {
        match dependency_type {
            DependencyType::Variable => {
                self.state.declarations.variables.get(name).map(|decl| decl.position)
            }
            DependencyType::Action => {
                self.state.declarations.actions.get(name).map(|decl| decl.position)
            }
        }
    }

    fn validate_action_types(&mut self) {
        let errors: Vec<_> = self.state.declarations.actions
            .iter()
            .filter(|(_, decl)| decl.spec.is_none())
            .filter_map(|(_, decl)| {
                validation_rules::validate_action(&decl.action_type, self.addon_specs).err()
            })
            .collect();

        errors.into_iter()
            .for_each(|error| self.add_error(error, Position::new(0, 0)));
    }

    fn validate_references(&mut self, body: &Body) {
        // Process all blocks collecting validation results
        let validation_results: Vec<_> = body.blocks()
            .map(|block| {
                let block_type = BlockType::from_str(block.ident.value());
                let current_entity = self.get_current_entity(block, block_type);

                // Validate action parameters if this is an action block
                let mut param_errors = Vec::new();
                if block_type == BlockType::Action {
                    param_errors = self.validate_action_parameters(block);
                }

                // Create visitor and collect validation data
                let handler = ValidationPhaseHandler {
                    state: &self.state,
                    source_mapper: &self.source_mapper,
                    file_path: &self.file_path,
                };

                let mut visitor = ReferenceValidationVisitor {
                    handler,
                    errors: Vec::new(),
                    input_refs: Vec::new(),
                    flow_input_refs: Vec::new(),
                    dependencies: Vec::new(),
                    current_entity: current_entity.clone(),
                    in_post_condition: false,
                };

                visitor.visit_block(block);

                (current_entity, visitor.errors, visitor.input_refs, visitor.flow_input_refs, visitor.dependencies, param_errors)
            })
            .collect();

        // Process all collected results
        validation_results.into_iter().for_each(|(current_entity, errors, input_refs, flow_input_refs, dependencies, param_errors)| {
            // Extend input references
            self.state.input_refs.extend(input_refs);

            // Collect flow input references, grouping by input name
            for flow_ref in flow_input_refs {
                self.state.flow_input_refs
                    .entry(flow_ref.input_name.clone())
                    .or_insert_with(Vec::new)
                    .push(flow_ref);
            }

            // Add dependency edges using pattern matching
            if let Some((entity_type, entity_name)) = current_entity {
                let graph = match entity_type {
                    EntityType::Variable => &mut self.state.dependency_graphs.variables,
                    EntityType::Action => &mut self.state.dependency_graphs.actions,
                };

                dependencies.into_iter()
                    .filter(|(dep_type, _)| match entity_type {
                        EntityType::Variable => dep_type == "variable",
                        EntityType::Action => dep_type == "action",
                    })
                    .for_each(|(_, dep_name)| graph.add_edge(&entity_name, dep_name));
            }

            // Add all errors
            errors.into_iter()
                .for_each(|(error, position)| self.add_error(error, position));

            // Add parameter validation errors
            param_errors.into_iter()
                .for_each(|(error, position)| self.add_error(error, position));
        });
    }

    fn get_current_entity(&self, block: &Block, block_type: BlockType) -> Option<(EntityType, String)> {
        match block_type {
            BlockType::Variable => {
                block.labels.get(0).and_then(|label| match label {
                    BlockLabel::String(s) => Some((EntityType::Variable, s.value().to_string())),
                    _ => None,
                })
            }
            BlockType::Action => {
                block.labels.get(0).and_then(|label| match label {
                    BlockLabel::String(s) => Some((EntityType::Action, s.value().to_string())),
                    _ => None,
                })
            }
            _ => None,
        }
    }

    fn add_error(&mut self, error: ValidationError, position: Position) {
        self.result.errors.push(LegacyError {
            message: error.to_string(),
            file: self.file_path.to_string(),
            line: Some(position.line),
            column: Some(position.column),
            context: None,
            related_locations: vec![],
            documentation_link: None,
        });
    }

    fn validate_action_parameters(&self, block: &Block) -> Vec<(ValidationError, Position)> {
        let mut errors = Vec::new();

        // Get action name and look up its spec
        let action_name = block.labels.get(0)
            .and_then(|label| match label {
                BlockLabel::String(s) => Some(s.value()),
                _ => None,
            });

        let action_type = block.labels.get(1)
            .and_then(|label| match label {
                BlockLabel::String(s) => Some(s.value()),
                _ => None,
            });

        if let (Some(name), Some(action_type)) = (action_name, action_type) {
            // Look up the action's command specification
            if let Some(action_decl) = self.state.declarations.actions.get(name) {
                if let Some(ref spec) = action_decl.spec {
                    // Collect all attribute names from the block (excluding inherited properties)
                    let mut block_params: HashSet<String> = block.body.attributes()
                        .filter(|attr| !validation_rules::is_inherited_property(attr.key.as_str()))
                        .map(|attr| attr.key.to_string())
                        .collect();

                    // Also collect block identifiers (for map-type parameters)
                    // These are parameters defined as blocks rather than attributes
                    // Filter out inherited properties like pre_condition and post_condition
                    block_params.extend(
                        block.body.blocks()
                            .filter(|b| !validation_rules::is_inherited_property(b.ident.as_str()))
                            .map(|b| b.ident.to_string())
                    );

                    // Collect valid input names from the spec
                    let valid_inputs: HashSet<String> = spec.inputs.iter()
                        .map(|input| input.name.clone())
                        .chain(spec.default_inputs.iter().map(|input| input.name.clone()))
                        .collect();

                    // Check for invalid parameters (not in spec)
                    let invalid_param_errors = block_params.iter()
                        .filter(|param_name| !valid_inputs.contains(*param_name) && !spec.accepts_arbitrary_inputs)
                        .map(|param_name| {
                            // Try to find position from attributes first
                            let position = block.body.attributes()
                                .find(|attr| attr.key.as_str() == param_name)
                                .and_then(|attr| attr.span())
                                .map(|span| source_mapper_to_position(&self.source_mapper, &span))
                                // If not found in attributes, try blocks
                                .or_else(|| {
                                    block.body.blocks()
                                        .find(|b| b.ident.as_str() == param_name)
                                        .and_then(|b| b.ident.span())
                                        .map(|span| source_mapper_to_position(&self.source_mapper, &span))
                                })
                                .unwrap_or_else(|| Position::new(0, 0));

                            (
                                ValidationError::InvalidParameter {
                                    param: param_name.clone(),
                                    action: action_type.to_string(),
                                },
                                position,
                            )
                        });

                    // Check for missing required parameters
                    let missing_param_errors = spec.inputs.iter()
                        .filter(|input| !input.optional && !block_params.contains(&input.name))
                        .map(|input| {
                            let position = optional_span_to_position(
                                &self.source_mapper,
                                block.ident.span().as_ref()
                            );

                            (
                                ValidationError::MissingParameter {
                                    param: input.name.clone(),
                                    action: action_type.to_string(),
                                },
                                position,
                            )
                        });

                    errors.extend(invalid_param_errors);
                    errors.extend(missing_param_errors);
                }
            }
        }

        errors
    }

    fn validate_all_flow_inputs(&mut self) {
        // Loop over each referenced input and partition flows by definition status
        let errors: Vec<LegacyError> = self.state.flow_input_refs.iter()
            .flat_map(|(input_name, references)| {
                // Partition flows into those that define the input and those that don't
                let (defining, missing): (Vec<_>, Vec<_>) = self.state.declarations.flows.iter()
                    .partition(|(_, def)| def.inputs.contains(input_name));

                self.generate_flow_input_errors(
                    input_name,
                    references,
                    &defining,
                    &missing
                )
            })
            .collect();

        // Add all errors to the result
        self.result.errors.extend(errors);
    }

    fn generate_flow_input_errors(
        &self,
        input_name: &str,
        references: &[FlowInputReference],
        defining: &[(&String, &FlowDeclaration)],
        missing: &[(&String, &FlowDeclaration)],
    ) -> Vec<LegacyError> {
        match (defining.is_empty(), missing.is_empty()) {
            (true, false) => {
                // All flows missing the input - errors at reference sites
                references.iter().map(|ref_loc| LegacyError {
                    message: format!("Undefined flow input '{}'", input_name),
                    file: ref_loc.file_path.clone(),
                    line: Some(ref_loc.location.line),
                    column: Some(ref_loc.location.column),
                    context: None,
                    related_locations: self.state.declarations.flows.iter()
                        .map(|(name, def)| crate::validation::types::RelatedLocation {
                            file: self.file_path.to_string(),
                            line: def.position.line,
                            column: def.position.column,
                            message: format!("Flow '{}' is missing input '{}'", name, input_name),
                        })
                        .collect(),
                    documentation_link: None,
                }).collect()
            },
            (false, false) => {
                // Some flows missing the input - bidirectional errors
                let ref_errors = references.iter().map(|ref_loc| LegacyError {
                    message: format!("Flow input '{}' not defined in all flows", input_name),
                    file: ref_loc.file_path.clone(),
                    line: Some(ref_loc.location.line),
                    column: Some(ref_loc.location.column),
                    context: None,
                    related_locations: missing.iter()
                        .map(|(name, def)| crate::validation::types::RelatedLocation {
                            file: self.file_path.to_string(),
                            line: def.position.line,
                            column: def.position.column,
                            message: format!("Missing in flow '{}'", name),
                        })
                        .collect(),
                    documentation_link: None,
                });

                let flow_errors = missing.iter().map(|(name, def)| {
                    let context_desc = match &references.first().map(|r| &r.context) {
                        Some(BlockContext::Action(action_name)) =>
                            format!("action '{}'", action_name),
                        Some(BlockContext::Variable(var_name)) =>
                            format!("variable '{}'", var_name),
                        Some(BlockContext::Output(output_name)) =>
                            format!("output '{}'", output_name),
                        Some(BlockContext::Flow(flow_name)) =>
                            format!("flow '{}'", flow_name),
                        Some(BlockContext::Signer(signer_name)) =>
                            format!("signer '{}'", signer_name),
                        Some(BlockContext::Addon(addon_name)) =>
                            format!("addon '{}'", addon_name),
                        Some(BlockContext::Unknown) | None => "unknown context".to_string(),
                    };

                    LegacyError {
                        message: format!("Flow '{}' missing input '{}'", name, input_name),
                        file: self.file_path.to_string(),
                        line: Some(def.position.line),
                        column: Some(def.position.column),
                        context: Some(format!("Input '{}' is referenced in {}", input_name, context_desc)),
                        related_locations: references.iter()
                            .map(|ref_loc| crate::validation::types::RelatedLocation {
                                file: ref_loc.file_path.clone(),
                                line: ref_loc.location.line,
                                column: ref_loc.location.column,
                                message: "Referenced here".to_string(),
                            })
                            .collect(),
                        documentation_link: None,
                    }
                });

                ref_errors.chain(flow_errors).collect()
            },
            _ => vec![], // All flows define the input - no errors
        }
    }
}


struct ReferenceValidationVisitor<'a> {
    handler: ValidationPhaseHandler<'a>,
    errors: Vec<(ValidationError, Position)>,
    input_refs: Vec<LocatedInputRef>,
    flow_input_refs: Vec<FlowInputReference>,
    dependencies: Vec<(String, String)>, // (type, name) pairs
    current_entity: Option<(EntityType, String)>,
    in_post_condition: bool, // Track if we're inside a post_condition block
}

impl<'a> Visit for ReferenceValidationVisitor<'a> {
    fn visit_block(&mut self, block: &Block) {
        // Track when entering/leaving post_condition blocks
        let was_in_post_condition = self.in_post_condition;
        let block_name = block.ident.as_str();

        if block_name == "post_condition" {
            self.in_post_condition = true;
        }

        // Visit the block's contents
        visit_block(self, block);

        // Restore the previous state
        self.in_post_condition = was_in_post_condition;
    }

    fn visit_expr(&mut self, expr: &Expression) {
        if let Expression::Traversal(traversal) = expr {
            let parts = extract_traversal_parts(traversal);
            let position = optional_span_to_position(
                self.handler.source_mapper,
                traversal.span().as_ref()
            );

            // Collect input references
            if parts.len() >= 2 && parts[0] == "input" {
                self.input_refs.push(LocatedInputRef {
                    name: parts[1].clone(),
                    line: position.line,
                    column: position.column,
                });
            }

            // Collect flow input references
            if parts.len() >= 2 && parts[0] == "flow" {
                let context = match &self.current_entity {
                    Some((EntityType::Action, name)) => BlockContext::Action(name.clone()),
                    Some((EntityType::Variable, name)) => BlockContext::Variable(name.clone()),
                    None => {
                        // Check if we're in an output or flow block by looking at current block type
                        // For now, default to a generic context - this will be refined
                        BlockContext::Action("unknown".to_string())
                    }
                };

                self.flow_input_refs.push(FlowInputReference {
                    input_name: parts[1].clone(),
                    location: position,
                    file_path: self.handler.file_path.to_string(),
                    context,
                });
            }

            // Track dependencies for circular dependency detection
            // Skip dependency tracking in post_condition blocks since they execute AFTER the action
            if !self.in_post_condition && parts.len() >= 2 {
                match parts[0].as_str() {
                    "var" | "variable" => {
                        self.dependencies.push(("variable".to_string(), parts[1].clone()));
                    }
                    "action" => {
                        self.dependencies.push(("action".to_string(), parts[1].clone()));
                    }
                    _ => {}
                }
            }

            if let Err(error) = self.handler.validate_reference(&parts, position) {
                self.errors.push((error, position));
            }
        }
        visit_expr(self, expr);
    }
}

fn extract_traversal_parts(traversal: &Traversal) -> Vec<String> {
    traversal.expr.as_variable()
        .map(|root| vec![root.to_string()])
        .unwrap_or_default()
        .into_iter()
        .chain(
            traversal.operators.iter()
                .filter_map(|op| match op.value() {
                    TraversalOperator::GetAttr(attr) => Some(attr.to_string()),
                    _ => None,
                })
        )
        .collect()
}

/// Basic HCL validator without addon support.
pub struct BasicHclValidator<'a> {
    result: &'a mut ValidationResult,
    file_path: &'a str,
    source: &'a str,
}

/// HCL validator with addon command specifications for parameter validation.
pub struct FullHclValidator<'a> {
    result: &'a mut ValidationResult,
    file_path: &'a str,
    source: &'a str,
    addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
}

impl<'a> BasicHclValidator<'a> {
    pub fn new(result: &'a mut ValidationResult, file_path: &'a str, source: &'a str) -> Self {
        Self { result, file_path, source }
    }

    pub fn validate(&mut self, body: &Body) -> Vec<LocatedInputRef> {
        // Create empty specs inline - no self-reference needed
        let empty_specs = HashMap::new();
        let mut validator = HclValidationVisitor::new(
            self.result,
            self.file_path,
            self.source,
            &empty_specs
        );
        validator.validate(body)
    }
}

impl<'a> FullHclValidator<'a> {
    pub fn new(
        result: &'a mut ValidationResult,
        file_path: &'a str,
        source: &'a str,
        addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        Self { result, file_path, source, addon_specs }
    }

    pub fn validate(&mut self, body: &Body) -> Vec<LocatedInputRef> {
        let mut validator = HclValidationVisitor::new(
            self.result,
            self.file_path,
            self.source,
            &self.addon_specs
        );
        validator.validate(body)
    }
}

pub fn validate_with_hcl(
    content: &str,
    result: &mut ValidationResult,
    file_path: &str,
) -> Result<Vec<LocatedInputRef>, String> {
    let body: Body = content.parse().map_err(|e| format!("Failed to parse: {}", e))?;
    let mut validator = BasicHclValidator::new(result, file_path, content);
    Ok(validator.validate(&body))
}

pub fn validate_with_hcl_and_addons(
    content: &str,
    result: &mut ValidationResult,
    file_path: &str,
    addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
) -> Result<Vec<LocatedInputRef>, String> {
    let body: Body = content.parse().map_err(|e| format!("Failed to parse: {}", e))?;
    let mut validator = FullHclValidator::new(result, file_path, content, addon_specs);
    Ok(validator.validate(&body))
}