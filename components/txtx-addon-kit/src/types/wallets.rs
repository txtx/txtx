use std::{future::Future, pin::Pin};

use hcl_edit::{expr::Expression, structure::Block};
use rust_fsm::StateMachine;

use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};

use super::{
    commands::{
        CommandExecutionContext, CommandExecutionFutureResult, CommandExecutionResult,
        CommandInput, CommandInputsEvaluationResult, CommandInstanceStateMachine, CommandOutput,
    },
    diagnostics::{Diagnostic, DiagnosticLevel},
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemResponseType, ActionItemStatus,
        ActionSubGroup,
    },
    types::{ObjectProperty, Type, Value},
    ConstructUuid, PackageUuid, ValueStore,
};

pub type WalletRunner = Box<
    fn(
        &ConstructUuid,
        &WalletSpecification,
        &ValueStore,
        &mut ValueStore,
        &AddonDefaults,
        &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult,
>;

pub type WalletSigner = Box<
    fn(
        &ConstructUuid,
        &str,
        &Value,
        &WalletSpecification,
        &ValueStore,
        &ValueStore,
        &AddonDefaults,
    ) -> CommandExecutionFutureResult,
>;

pub type WalletActivabilityChecker = fn(
    &ConstructUuid,
    &str,
    &WalletSpecification,
    &ValueStore,
    &mut ValueStore,
    &AddonDefaults,
    &CommandExecutionContext,
) -> WalletActivabilityFutureResult;

pub type WalletActivabilityFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<Vec<ActionSubGroup>, Diagnostic>> + Send>>,
    Diagnostic,
>;

pub type WalletInstantiabilityChecker =
    fn(&WalletSpecification, Vec<Type>) -> Result<Type, Diagnostic>;

pub type WalletSignabilityChecker = fn(
    &ConstructUuid,
    &str,
    &Value,
    &WalletSpecification,
    &ValueStore,
    &ValueStore,
    &AddonDefaults,
    &CommandExecutionContext,
) -> Result<Vec<ActionItemRequest>, Diagnostic>;

pub type WalletPublicKeyExpectations = fn(
    &ConstructUuid,
    &str,
    &Vec<u8>,
    &WalletSpecification,
    &ValueStore,
    &AddonDefaults,
    &CommandExecutionContext,
) -> Result<Option<String>, Diagnostic>;

#[derive(Debug, Clone)]
pub struct WalletSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub check_instantiability: WalletInstantiabilityChecker,
    pub check_activability: WalletActivabilityChecker,
    pub activate: WalletRunner,
    pub check_signability: WalletSignabilityChecker,
    pub sign: WalletSigner,
    pub check_public_key_expectations: WalletPublicKeyExpectations,
}

#[derive(Debug, Clone)]
pub struct WalletInstance {
    pub specification: WalletSpecification,
    pub state: StateMachine<CommandInstanceStateMachine>,
    pub store: ValueStore,
    pub name: String,
    pub block: Block,
    pub package_uuid: PackageUuid,
    pub namespace: String,
}

impl WalletInstance {
    pub fn check_inputs(&self) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let mut has_errors = false;

        for input in self.specification.inputs.iter() {
            match (input.optional, self.block.body.get_attribute(&input.name)) {
                (false, None) => {
                    has_errors = true;
                    diagnostics.push(Diagnostic::error_from_expression(
                        &self.block,
                        None,
                        format!("missing attribute '{}'", input.name),
                    ));
                }
                (_, Some(_attr)) => {
                    // todo(lgalabru): check typing
                }
                (_, _) => {}
            }
        }

        // todo(lgalabru): check arbitrary attributes

        if has_errors {
            Err(diagnostics)
        } else {
            Ok(diagnostics)
        }
    }

    pub fn get_expressions_referencing_commands_from_inputs(
        &self,
    ) -> Result<Vec<Expression>, String> {
        let mut expressions = vec![];
        for input in self.specification.inputs.iter() {
            match input.typing {
                Type::Object(ref props) => {
                    for prop in props.iter() {
                        let mut blocks_iter = self.block.body.get_blocks(&input.name);
                        while let Some(block) = blocks_iter.next() {
                            let res = visit_optional_untyped_attribute(&prop.name, &block)
                                .map_err(|e| format!("{:?}", e))?;
                            if let Some(expr) = res {
                                let mut references = vec![];
                                collect_constructs_references_from_expression(
                                    &expr,
                                    &mut references,
                                );
                                expressions.append(&mut references);
                            }
                        }
                    }
                }
                _ => {
                    let res = visit_optional_untyped_attribute(&input.name, &self.block)
                        .map_err(|e| format!("{:?}", e))?;
                    if let Some(expr) = res {
                        let mut references = vec![];
                        collect_constructs_references_from_expression(&expr, &mut references);
                        expressions.append(&mut references);
                    }
                }
            }
        }
        Ok(expressions)
    }

    /// Checks the `CommandInstance` HCL Block for an attribute named `input.name`
    pub fn get_expression_from_input(
        &self,
        input: &CommandInput,
    ) -> Result<Option<Expression>, Diagnostic> {
        let res = match &input.typing {
            Type::Primitive(_) | Type::Array(_) | Type::Addon(_) => {
                visit_optional_untyped_attribute(&input.name, &self.block)?
            }
            Type::Object(_) => unreachable!(),
        };
        match (res, input.optional) {
            (Some(res), _) => Ok(Some(res)),
            (None, true) => Ok(None),
            (None, false) => todo!(
                "command '{}' (type '{}') is missing value for field '{}'",
                self.name,
                self.specification.matcher,
                input.name
            ),
        }
    }

    pub fn get_group(&self) -> String {
        let Some(group) = self.block.body.get_attribute("group") else {
            return format!("{} Review", self.specification.name.to_string());
        };
        group.value.to_string()
    }

    pub fn get_expression_from_object_property(
        &self,
        input: &CommandInput,
        prop: &ObjectProperty,
    ) -> Result<Option<Expression>, Diagnostic> {
        let object = self.block.body.get_blocks(&input.name).next();
        match (object, input.optional) {
            (Some(block), _) => {
                let expr_res = visit_optional_untyped_attribute(&prop.name, &block)?;
                match (expr_res, prop.optional) {
                    (Some(expression), _) => Ok(Some(expression)),
                    (None, true) => Ok(None),
                    (None, false) => todo!(
                        "command '{}' (type '{}') is missing property '{}' for object '{}'",
                        self.name,
                        self.specification.matcher,
                        prop.name,
                        input.name
                    ),
                }
            }
            (None, true) => Ok(None),
            (None, false) => todo!(
                "command '{}' (type '{}') is missing object '{}'",
                self.name,
                self.specification.matcher,
                input.name
            ),
        }
    }

    pub async fn check_activability(
        &mut self,
        construct_uuid: &ConstructUuid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        addon_defaults: AddonDefaults,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        action_item_responses: &Option<&Vec<ActionItemResponseType>>,
        execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionSubGroup>, Diagnostic> {
        let mut values = ValueStore::new(&self.name);
        for input in self.specification.inputs.iter() {
            let value = match input_evaluation_results.inputs.get(&input.name) {
                Some(Ok(value)) => Ok(value.clone()),
                Some(Err(e)) => Err(Diagnostic {
                    span: None,
                    location: None,
                    message: format!("Cannot execute command due to erroring inputs"),
                    level: DiagnosticLevel::Error,
                    documentation: None,
                    example: None,
                    parent_diagnostic: Some(Box::new(e.clone())),
                }),
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo: return diagnostic
                },
            }
            .unwrap();
            values.insert(&input.name, value);
        }

        match action_item_responses {
            Some(responses) => responses.iter().for_each(|response| match response {
                ActionItemResponseType::ProvidePublicKey(update) => {
                    let public_key_bytes =
                        hex::decode(&update.public_key).expect("unable to decode bytes");

                    input_evaluation_results.inputs.insert(
                        "public_key".into(),
                        Ok(Value::string(update.public_key.clone())),
                    );
                    for input in self.specification.inputs.iter_mut() {
                        if input.name.eq("public_key") {
                            input.check_performed = true;
                            break;
                        }
                    }

                    let res = ((&self.specification).check_public_key_expectations)(
                        &construct_uuid,
                        &self.name,
                        &public_key_bytes,
                        &self.specification,
                        &values,
                        &addon_defaults,
                        &execution_context,
                    );

                    self.store
                        .insert("public_key", Value::string(update.public_key.clone()));

                    for request in action_item_requests.iter_mut() {
                        let (status, success) = match &res {
                            Ok(message) => (ActionItemStatus::Success(message.clone()), true),
                            Err(diag) => (ActionItemStatus::Error(diag.clone()), false),
                        };

                        match request.action_type {
                            ActionItemRequestType::ReviewInput => {
                                request.action_status = status.clone();
                            }
                            ActionItemRequestType::ProvidePublicKey(_) => {
                                if success {
                                    request.action_status = status.clone();
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                _ => {}
            }),
            None => {}
        }

        let spec = &self.specification;
        (spec.check_activability)(
            &construct_uuid,
            &self.name,
            &self.specification,
            &values,
            &mut self.store,
            &addon_defaults,
            &execution_context,
        )?
        .await
    }

    pub async fn perform_activation(
        &mut self,
        construct_uuid: &ConstructUuid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        addon_defaults: AddonDefaults,
        progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        // todo: I don't think this one needs to be a result
        let mut values = ValueStore::new(&self.name);
        for input in self.specification.inputs.iter() {
            let value = match evaluated_inputs.inputs.get(&input.name) {
                Some(Ok(value)) => Ok(value.clone()),
                Some(Err(e)) => Err(Diagnostic {
                    span: None,
                    location: None,
                    message: format!("Cannot execute command due to erroring inputs"),
                    level: DiagnosticLevel::Error,
                    documentation: None,
                    example: None,
                    parent_diagnostic: Some(Box::new(e.clone())),
                }),
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo(lgalabru): return diagnostic
                },
            }?;
            values.insert(&input.name, value);
        }
        (&self.specification.activate)(
            &construct_uuid,
            &self.specification,
            &values,
            &mut self.store,
            &addon_defaults,
            progress_tx,
        )?
        .await
    }
}

pub trait WalletImplementation {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    // todo: should potentially merge in check_instantiability
    fn check_public_key_expectations(
        _uuid: &ConstructUuid,
        instance_name: &str,
        public_key_bytes: &Vec<u8>,
        spec: &WalletSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Option<String>, Diagnostic>;

    fn check_activability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult;

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult;

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        Ok(vec![])
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &ValueStore,
        _defaults: &AddonDefaults,
    ) -> CommandExecutionFutureResult;
}
