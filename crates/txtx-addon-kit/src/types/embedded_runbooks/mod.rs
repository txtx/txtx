use std::collections::{HashMap, HashSet};

use hcl_edit::{expr::Expression, structure::Block};
use url::Url;

use crate::helpers::{
    fs::FileLocation,
    hcl::{
        collect_constructs_references_from_expression, get_object_expression_key,
        visit_optional_untyped_attribute, RawHclContent,
    },
};

use super::{
    commands::{CommandInput, CommandInstance},
    diagnostics::Diagnostic,
    package::Package,
    signers::{SignerInstance, SignersState},
    stores::ValueStore,
    types::{ObjectProperty, Type},
    AddonInstance, ConstructDid, ConstructId, EvaluatableInput, PackageId, RunbookId,
    WithEvaluatableInputs,
};

#[derive(Debug, Clone)]
pub struct EmbeddedRunbookInstance {
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
    pub specification: EmbeddedRunbookInstanceSpecification,
}

impl WithEvaluatableInputs for EmbeddedRunbookInstance {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn get_expression_from_input(&self, input_name: &str) -> Option<Expression> {
        visit_optional_untyped_attribute(&input_name, &self.block)
    }

    fn get_blocks_for_map(
        &self,
        input_name: &str,
        input_typing: &Type,
        input_optional: bool,
    ) -> Result<Vec<Block>, Vec<super::diagnostics::Diagnostic>> {
        let mut entries = vec![];

        match &input_typing {
            Type::Map(_) => {
                for block in self.block.body.get_blocks(&input_name) {
                    entries.push(block.clone());
                }
            }
            _ => {
                unreachable!()
            }
        };
        if entries.is_empty() && !input_optional {
            return Err(vec![Diagnostic::error_from_string(format!(
                "embedded runbook '{}' is missing value for object '{}'",
                self.name, input_name
            ))]);
        }
        Ok(entries)
    }

    fn get_expression_from_block(
        &self,
        block: &Block,
        prop: &ObjectProperty,
    ) -> Option<Expression> {
        visit_optional_untyped_attribute(&prop.name, &block)
    }

    fn get_expression_from_object(
        &self,
        input_name: &str,
        input_typing: &Type,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        match &input_typing {
            Type::Object(_) => Ok(visit_optional_untyped_attribute(&input_name, &self.block)),
            _ => Err(vec![Diagnostic::error_from_string(format!(
                "embedded runbook '{}' expected object for input '{}'",
                self.name, input_name
            ))]),
        }
    }

    fn get_expression_from_object_property(
        &self,
        input_name: &str,
        prop: &super::types::ObjectProperty,
    ) -> Option<Expression> {
        let expr = visit_optional_untyped_attribute(&input_name, &self.block);
        match expr {
            Some(expr) => {
                let object_expr = expr.as_object().unwrap();
                let expr_res = get_object_expression_key(object_expr, &prop.name);
                match expr_res {
                    Some(expression) => Some(expression.expr().clone()),
                    None => None,
                }
            }
            None => None,
        }
    }

    fn spec_inputs(&self) -> Vec<impl EvaluatableInput> {
        self.specification.inputs.iter().filter_map(|i| i.as_value()).collect()
    }
}

impl EmbeddedRunbookInstance {
    pub fn new(
        name: &str,
        block: &Block,
        package_id: &PackageId,
        specification: EmbeddedRunbookInstanceSpecification,
    ) -> Self {
        Self {
            name: name.to_string(),
            block: block.clone(),
            package_id: package_id.clone(),
            specification,
        }
    }

    pub fn get_expressions_referencing_commands_from_runbook_inputs(
        &self,
    ) -> Result<Vec<(Option<&EmbeddedRunbookInputSpecification>, Expression)>, String> {
        let mut expressions = vec![];
        for input in self.specification.inputs.iter() {
            match input {
                EmbeddedRunbookInputSpecification::Value(value_spec) => match value_spec.typing {
                    Type::Map(ref props) => {
                        for block in self.block.body.get_blocks(&value_spec.name) {
                            for prop in props.iter() {
                                let res = visit_optional_untyped_attribute(&prop.name, &block);
                                if let Some(expr) = res {
                                    let mut references = vec![];
                                    collect_constructs_references_from_expression(
                                        &expr,
                                        Some(input),
                                        &mut references,
                                    );
                                    expressions.append(&mut references);
                                }
                            }
                        }
                    }
                    Type::Object(ref props) => {
                        let res = visit_optional_untyped_attribute(&value_spec.name, &self.block);
                        if let Some(expr) = res {
                            let mut references = vec![];
                            collect_constructs_references_from_expression(
                                &expr,
                                Some(input),
                                &mut references,
                            );
                            expressions.append(&mut references);
                        }
                        for prop in props.iter() {
                            let mut blocks_iter = self.block.body.get_blocks(&value_spec.name);
                            while let Some(block) = blocks_iter.next() {
                                let res = visit_optional_untyped_attribute(&prop.name, &block);
                                if let Some(expr) = res {
                                    let mut references = vec![];
                                    collect_constructs_references_from_expression(
                                        &expr,
                                        Some(input),
                                        &mut references,
                                    );
                                    expressions.append(&mut references);
                                }
                            }
                        }
                    }
                    _ => {
                        let res = visit_optional_untyped_attribute(&value_spec.name, &self.block);
                        if let Some(expr) = res {
                            let mut references = vec![];
                            collect_constructs_references_from_expression(
                                &expr,
                                Some(input),
                                &mut references,
                            );
                            expressions.append(&mut references);
                        }
                    }
                },
                EmbeddedRunbookInputSpecification::Signer(signer_spec) => {
                    let res = visit_optional_untyped_attribute(&signer_spec.name, &self.block);
                    if let Some(expr) = res {
                        let mut references = vec![];
                        collect_constructs_references_from_expression(
                            &expr,
                            Some(input),
                            &mut references,
                        );
                        expressions.append(&mut references);
                    }
                }
            }
        }
        Ok(expressions)
    }

    pub fn collect_dependencies(
        &self,
    ) -> Vec<(Option<&EmbeddedRunbookInputSpecification>, Expression)> {
        let mut dependencies = vec![];
        for input in self.specification.inputs.iter() {
            match input {
                EmbeddedRunbookInputSpecification::Value(value_spec) => match value_spec.typing {
                    Type::Object(ref props) => {
                        if let Some(attr) = self.block.body.get_attribute(&value_spec.name) {
                            collect_constructs_references_from_expression(
                                &attr.value,
                                Some(input),
                                &mut dependencies,
                            );
                        } else {
                            for prop in props.iter() {
                                let mut blocks_iter = self.block.body.get_blocks(&value_spec.name);
                                while let Some(block) = blocks_iter.next() {
                                    let Some(attr) = block.body.get_attribute(&prop.name) else {
                                        continue;
                                    };
                                    collect_constructs_references_from_expression(
                                        &attr.value,
                                        Some(input),
                                        &mut dependencies,
                                    );
                                }
                            }
                        }
                    }
                    _ => {
                        let Some(attr) = self.block.body.get_attribute(&value_spec.name) else {
                            continue;
                        };
                        collect_constructs_references_from_expression(
                            &attr.value,
                            Some(input),
                            &mut dependencies,
                        );
                    }
                },
                EmbeddedRunbookInputSpecification::Signer(signer_spec) => {
                    let res = visit_optional_untyped_attribute(&signer_spec.name, &self.block);
                    if let Some(expr) = res {
                        collect_constructs_references_from_expression(
                            &expr,
                            Some(input),
                            &mut dependencies,
                        );
                    }
                }
            }
        }
        dependencies
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedRunbookInstanceSpecification {
    pub runbook_id: RunbookId,
    pub description: Option<String>,
    pub hcl: RawHclContent,
    pub inputs: Vec<EmbeddedRunbookInputSpecification>,
    pub static_execution_context: EmbeddedRunbookStaticExecutionContext,
    pub static_workspace_context: EmbeddedRunbookStaticWorkspaceContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddedRunbookInputSpecification {
    Value(EmbeddedRunbookValueInputSpecification),
    Signer(EmbeddedRunbookSignerInputSpecification),
}

impl EvaluatableInput for EmbeddedRunbookInputSpecification {
    fn name(&self) -> String {
        match self {
            EmbeddedRunbookInputSpecification::Value(value_spec) => value_spec.name.clone(),
            EmbeddedRunbookInputSpecification::Signer(signer_spec) => signer_spec.name.clone(),
        }
    }

    fn typing(&self) -> &Type {
        match self {
            EmbeddedRunbookInputSpecification::Value(value_spec) => &value_spec.typing,
            EmbeddedRunbookInputSpecification::Signer(_) => todo!(),
        }
    }

    fn optional(&self) -> bool {
        false
    }
}

impl EmbeddedRunbookInputSpecification {
    pub fn as_value(&self) -> Option<Self> {
        match self {
            EmbeddedRunbookInputSpecification::Value(value_spec) => {
                Some(EmbeddedRunbookInputSpecification::Value(value_spec.clone()))
            }
            EmbeddedRunbookInputSpecification::Signer(_) => None,
        }
    }

    pub fn new_value(name: &String, typing: &Type, documentation: &String) -> Self {
        EmbeddedRunbookInputSpecification::Value(EmbeddedRunbookValueInputSpecification {
            name: name.clone(),
            documentation: documentation.clone(),
            typing: typing.clone(),
        })
    }
    pub fn from_command_input(command_input: &CommandInput) -> Self {
        EmbeddedRunbookInputSpecification::Value(EmbeddedRunbookValueInputSpecification {
            name: command_input.name.clone(),
            documentation: command_input.documentation.clone(),
            typing: command_input.typing.clone(),
        })
    }
    pub fn from_signer_instance(signer: &SignerInstance) -> Self {
        EmbeddedRunbookInputSpecification::Signer(EmbeddedRunbookSignerInputSpecification {
            name: signer.name.clone(),
            documentation: String::new(),
            namespace: signer.namespace.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedRunbookValueInputSpecification {
    pub name: String,
    pub documentation: String,
    pub typing: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedRunbookSignerInputSpecification {
    pub name: String,
    pub documentation: String,
    pub namespace: String,
}

pub type SignerName = String;
#[derive(Debug, Clone)]
pub struct EmbeddedRunbookStaticExecutionContext {
    /// Map of addon instances (addon "evm" { ... })
    pub addon_instances: HashMap<ConstructDid, AddonInstance>,
    /// Map of embedded runbooks
    pub embedded_runbooks: HashMap<ConstructDid, EmbeddedRunbookInstance>,
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, CommandInstance>,
    /// Constructs depending on a given Construct.
    pub commands_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing.
    /// The signer is delineated by the name of the signer as used by the embedded runbook.
    pub signers_downstream_dependencies: Vec<(SignerName, Vec<ConstructDid>)>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands_upstream_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands: HashSet<ConstructDid>,
    /// Commands execution order
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order
    pub order_for_signers_initialization: Vec<ConstructDid>,
    /// Published evaluated inputs
    pub evaluated_inputs: ValueStore,
}

#[derive(Debug, Clone)]
pub struct EmbeddedRunbookStaticWorkspaceContext {
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    pub packages: HashMap<PackageId, Package>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, signer, import, ...)
    pub constructs: HashMap<ConstructDid, ConstructId>,
}

#[derive(Debug, Clone)]
pub struct EmbeddedRunbookStatefulExecutionContext {
    pub signer_did_lookup: HashMap<SignerName, ConstructDid>,
    pub signers_instances: HashMap<ConstructDid, SignerInstance>,
    pub signers_state: Option<SignersState>,
    pub signers_construct_id_lookup: HashMap<ConstructDid, ConstructId>,
}

impl EmbeddedRunbookStatefulExecutionContext {
    pub fn new(
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers_state: &Option<SignersState>,
        signers_construct_id_lookup: &HashMap<ConstructDid, ConstructId>,
    ) -> Self {
        let signer_did_lookup = signers_instances
            .iter()
            .map(|(did, signer_instance)| (signer_instance.name.clone(), did.clone()))
            .collect();

        Self {
            signer_did_lookup,
            signers_instances: signers_instances.clone(),
            signers_state: signers_state.clone(),
            signers_construct_id_lookup: signers_construct_id_lookup.clone(),
        }
    }
}