use rust_fsm::state_machine;
use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use std::{
    collections::HashMap,
    future::{self, Future},
    hash::Hash,
    pin::Pin,
};

use hcl_edit::{expr::Expression, structure::Block};

use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};

use super::{
    diagnostics::{Diagnostic, DiagnosticLevel},
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemResponseType, ActionItemStatus,
        Actions, BlockEvent,
    },
    types::{ObjectProperty, Type, TypeSpecification, Value},
    wallets::{WalletInstance, WalletSignFutureResult, WalletsState},
    ConstructUuid, PackageUuid, ValueStore,
};

#[derive(Clone, Debug)]
pub struct CommandExecutionContext {
    pub review_input_default_values: bool,
    pub review_input_values: bool,
}

#[derive(Clone, Debug)]
pub struct CommandExecutionResult {
    pub outputs: HashMap<String, Value>, // todo: change value to be Result<Value, Diagnostic>
}

impl Serialize for CommandExecutionResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.outputs.len()))?;
        for (k, v) in self.outputs.iter() {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}
impl CommandExecutionResult {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandInputsEvaluationResult {
    pub inputs: HashMap<String, Result<Value, Diagnostic>>, // todo(lgalabru): replace Value with EvaluatedExpression
}

impl Serialize for CommandInputsEvaluationResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inputs.len()))?;
        for (k, v) in self.inputs.iter() {
            let value = match v {
                Ok(v) => Some(v),
                Err(_) => None,
            };
            map.serialize_entry(&k, &value)?;
        }
        map.end()
    }
}

impl CommandInputsEvaluationResult {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: Result<Value, Diagnostic>) {
        self.inputs.insert(key.to_string(), value);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CommandInput {
    pub name: String,
    pub documentation: String,
    pub typing: Type,
    pub optional: bool,
    pub interpolable: bool,
    pub check_required: bool,
    pub check_performed: bool,
}

impl CommandInput {
    pub fn as_object(&self) -> Option<&Vec<ObjectProperty>> {
        match &self.typing {
            Type::Object(spec) => Some(spec),
            Type::Primitive(_) => None,
            Type::Addon(_) => None,
            Type::Array(_) => None,
        }
    }
    pub fn as_array(&self) -> Option<&Box<Type>> {
        match &self.typing {
            Type::Object(_) => None,
            Type::Primitive(_) => None,
            Type::Addon(_) => None,
            Type::Array(array) => Some(array),
        }
    }
    pub fn as_action(&self) -> Option<&TypeSpecification> {
        match &self.typing {
            Type::Object(_) => None,
            Type::Primitive(_) => None,
            Type::Addon(addon) => Some(addon),
            Type::Array(_) => None,
        }
    }
}

impl Serialize for CommandInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandInput", 4)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("documentation", &self.documentation)?;
        ser.serialize_field("typing", &self.typing)?;
        ser.serialize_field("optional", &self.optional)?;
        ser.end()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandOutput {
    pub name: String,
    pub documentation: String,
    pub typing: Type,
}

impl Serialize for CommandOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandOutput", 4)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("documentation", &self.documentation)?;
        ser.serialize_field("typing", &self.typing)?;
        ser.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CommandId {
    Action(String),
}

impl CommandId {
    pub fn to_string(&self) -> String {
        match &self {
            &CommandId::Action(id) => format!("action::{id}"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CommandInstanceOrParts {
    Instance(CommandInstance),
    Parts(Vec<String>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PreCommandSpecification {
    Atomic(CommandSpecification),
    Composite(CompositeCommandSpecification),
}

impl PreCommandSpecification {
    pub fn expect_atomic_specification(&self) -> &CommandSpecification {
        match &self {
            PreCommandSpecification::Atomic(spec) => spec,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub accepts_arbitrary_inputs: bool,
    pub create_output_for_each_input: bool,
    pub update_addon_defaults: bool,
    pub requires_signing_capability: bool,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub check_instantiability: InstantiabilityChecker,
    pub check_executability: CommandCheckExecutabilityClosure,
    pub run_execution: CommandExecutionClosure,
    pub check_signed_executability: CommandCheckSignedExecutabilityClosure,
    pub run_signed_execution: CommandSignedExecutionClosure,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompositeCommandSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub parts: Vec<PreCommandSpecification>,
    pub default_inputs: Vec<CommandInput>,
    pub router: CommandRouter,
    pub example: String,
}

impl CommandSpecification {
    pub fn default_inputs() -> Vec<CommandInput> {
        vec![
            CommandInput {
                name: "description".into(),
                documentation: "Allows you to describe and comment steps of your runbook".into(),
                typing: Type::string(),
                optional: true,
                interpolable: true,
                check_performed: false,
                check_required: false,
            },
            CommandInput {
                name: "labels".into(),
                documentation: "Allows you to label steps of your runbook".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: true,
                check_performed: false,
                check_required: false,
            },
            CommandInput {
                name: "environments".into(),
                documentation: "Only enable command for given environments (default: all)".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: false,
                check_performed: false,
                check_required: false,
            },
            CommandInput {
                name: "redacted".into(),
                documentation: "Never include value in logs".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: false,
                check_performed: false,
                check_required: false,
            },
            CommandInput {
                name: "group".into(),
                documentation: "Name used for grouping commands together".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: true,
                check_performed: false,
                check_required: false,
            },
        ]
    }
}

impl Serialize for CommandSpecification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandSpecification", 4)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("documentation", &self.documentation)?;
        ser.serialize_field("inputs", &self.inputs)?;
        ser.serialize_field("outputs", &self.outputs)?;
        ser.end()
    }
}

impl Serialize for CompositeCommandSpecification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // todo
        let mut ser = serializer.serialize_struct("CompositeCommandSpecification", 2)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("documentation", &self.documentation)?;
        ser.end()
    }
}

pub type InstantiabilityChecker = fn(&CommandSpecification, Vec<Type>) -> Result<Type, Diagnostic>;
pub type CommandExecutionFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>> + Send>>,
    Diagnostic,
>;

pub type CommandExecutionClosure = Box<
    fn(
        &ConstructUuid,
        &CommandSpecification,
        &ValueStore,
        &AddonDefaults,
        &channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult,
>;

pub type CommandSignedExecutionClosure = Box<
    fn(
        &ConstructUuid,
        &CommandSpecification,
        &ValueStore,
        &AddonDefaults,
        &channel::Sender<BlockEvent>,
        &HashMap<ConstructUuid, WalletInstance>,
        WalletsState,
    ) -> WalletSignFutureResult,
>;

type CommandRouter =
    fn(&String, &String, &Vec<PreCommandSpecification>) -> Result<Vec<String>, Diagnostic>;

pub type CommandCheckExecutabilityClosure = fn(
    &ConstructUuid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &AddonDefaults,
    &CommandExecutionContext,
) -> Result<Actions, Diagnostic>;

pub type CommandCheckSignedExecutabilityClosure =
    fn(
        &ConstructUuid,
        &str,
        &CommandSpecification,
        &ValueStore,
        &AddonDefaults,
        &CommandExecutionContext,
        &HashMap<ConstructUuid, WalletInstance>,
        WalletsState,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)>;

pub fn return_synchronous_result(
    res: Result<CommandExecutionResult, Diagnostic>,
) -> CommandExecutionFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_ok(res: CommandExecutionResult) -> CommandExecutionFutureResult {
    return_synchronous_result(Ok(res))
}

pub fn return_synchronous_err(diag: Diagnostic) -> CommandExecutionFutureResult {
    return_synchronous_result(Err(diag))
}

pub trait CompositeCommandImplementation {
    fn router(
        _first_input_body: &String,
        _command_instance_name: &String,
        _parts: &Vec<PreCommandSpecification>,
    ) -> Result<Vec<String>, Diagnostic>;
}

state_machine! {
  derive(Debug, Clone, Serialize)
  pub CommandInstanceStateMachine(New)

  New => {
    Successful => Evaluated,
    NeedsUserInput => AwaitingUserInput,
    NeedsAsyncRequest => AwaitingAsyncRequest,
    Unsuccessful => Failed,
  },
  AwaitingUserInput => {
    Successful => Evaluated,
    NeedsUserInput => AwaitingUserInput,
    Unsuccessful => Failed,
    Abort => Aborted,
    ReEvaluate => New
  },
  AwaitingAsyncRequest => {
    Successful => Evaluated,
    Unsuccessful => Failed,
    Abort => Aborted,
    ReEvaluate => New
  },
  Evaluated => {
    ReEvaluate => New,
    Successful => Evaluated
  },
  Aborted => {
    ReEvaluate => New
  },
  Failed => {
    ReEvaluate => New
  }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandInstanceType {
    Input,
    Output,
    Action,
    Prompt,
    Module,
}

#[derive(Debug, Clone)]
pub struct CommandInstance {
    pub specification: CommandSpecification,
    pub name: String,
    pub block: Block,
    pub package_uuid: PackageUuid,
    pub namespace: String,
    pub typing: CommandInstanceType,
}
pub enum CommandExecutionStatus {
    Complete(Result<CommandExecutionResult, Diagnostic>),
    NeedsAsyncRequest,
}

impl Serialize for CommandInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandInstance", 6)?;
        ser.serialize_field("specification", &self.specification)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("packageUuid", &self.package_uuid)?;
        ser.serialize_field("namespace", &self.namespace)?;
        ser.serialize_field("typing", &self.typing)?;
        ser.end()
    }
}

impl CommandInstance {
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
        if self.specification.accepts_arbitrary_inputs {
            for attribute in self.block.body.attributes() {
                let mut references = vec![];
                collect_constructs_references_from_expression(&attribute.value, &mut references);
                expressions.append(&mut references);
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

    pub fn check_executability(
        &mut self,
        construct_uuid: &ConstructUuid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        addon_defaults: AddonDefaults,
        _wallet_instances: &mut HashMap<ConstructUuid, WalletInstance>,
        action_item_response: &Option<&Vec<ActionItemResponseType>>,
        execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        let mut values = ValueStore::new(
            &format!("{}_inputs", self.specification.matcher),
            &construct_uuid.value(),
        );

        // TODO
        match action_item_response {
            Some(responses) => responses.into_iter().for_each(|response| match response {
                ActionItemResponseType::ReviewInput(update) => {
                    for input in self.specification.inputs.iter_mut() {
                        if input.name == update.input_name {
                            input.check_performed = update.value_checked;
                            break;
                        }
                    }
                }
                ActionItemResponseType::ProvideInput(update) => {
                    input_evaluation_results
                        .inputs
                        .insert(update.input_name.clone(), Ok(update.updated_value.clone()));
                    // todo: when there is a provide input update, we need to actually send an updated action item.
                    // the tricky part is we need to know the uuid of the original action we're updating
                }
                ActionItemResponseType::ProvideSignedTransaction(bytes) => {
                    // TODO
                    values.insert(
                        "signed_transaction_bytes",
                        Value::string(bytes.signed_transaction_bytes.clone()),
                    );
                }
                _ => {}
            }),
            None => {}
        }

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
                    false => unreachable!("{} missing?", input.name), // todo: return diagnostic
                },
            }
            .unwrap();
            values.insert(&input.name, value);
        }

        let spec = &self.specification;
        (spec.check_executability)(
            &construct_uuid,
            &self.name,
            &self.specification,
            &values,
            &addon_defaults,
            &execution_context,
        )
    }

    pub async fn perform_execution(
        &self,
        construct_uuid: &ConstructUuid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        addon_defaults: AddonDefaults,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponseType>>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut values = ValueStore::new(&self.name, &construct_uuid.value());
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

        let spec = &self.specification;
        let res = (spec.run_execution)(
            &construct_uuid,
            &self.specification,
            &values,
            &addon_defaults,
            progress_tx,
        )?
        .await;

        for request in action_item_requests.iter_mut() {
            let (status, success) = match &res {
                Ok(_) => (ActionItemStatus::Success(None), true),
                Err(diag) => (ActionItemStatus::Error(diag.clone()), false),
            };
            match request.action_type {
                ActionItemRequestType::ReviewInput(_) => {
                    request.action_status = status.clone();
                }
                ActionItemRequestType::ProvidePublicKey(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                ActionItemRequestType::ProvideSignedTransaction(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                _ => unreachable!(),
            }
        }
        res
    }

    pub fn check_signed_executability(
        &mut self,
        construct_uuid: &ConstructUuid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        wallets: WalletsState,
        addon_defaults: AddonDefaults,
        wallet_instances: &mut HashMap<ConstructUuid, WalletInstance>,
        action_item_response: &Option<&Vec<ActionItemResponseType>>,
        execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        let mut values = ValueStore::new(
            &format!("{}_inputs", self.specification.matcher),
            &construct_uuid.value(),
        );

        // TODO
        match action_item_response {
            Some(responses) => responses.into_iter().for_each(|response| match response {
                ActionItemResponseType::ReviewInput(update) => {
                    for input in self.specification.inputs.iter_mut() {
                        if input.name == update.input_name {
                            input.check_performed = true;
                            break;
                        }
                    }
                }
                ActionItemResponseType::ProvideInput(update) => {
                    input_evaluation_results
                        .inputs
                        .insert(update.input_name.clone(), Ok(update.updated_value.clone()));
                    for input in self.specification.inputs.iter_mut() {
                        if input.name == update.input_name {
                            input.check_performed = true;
                            break;
                        }
                    }
                }
                ActionItemResponseType::ProvideSignedTransaction(bytes) => {
                    // TODO
                    values.insert(
                        "signed_transaction_bytes",
                        Value::string(bytes.signed_transaction_bytes.clone()),
                    );
                }
                _ => {}
            }),
            None => {}
        }

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
                    false => unreachable!("{} missing?", input.name), // todo: return diagnostic
                },
            }
            .unwrap();
            values.insert(&input.name, value);
        }

        let spec = &self.specification;
        (spec.check_signed_executability)(
            &construct_uuid,
            &self.name,
            &self.specification,
            &values,
            &addon_defaults,
            &execution_context,
            wallet_instances,
            wallets,
        )
    }

    pub async fn perform_signed_execution(
        &self,
        construct_uuid: &ConstructUuid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        wallets: WalletsState,
        addon_defaults: AddonDefaults,
        wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponseType>>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)> {
        let mut values = ValueStore::new(&self.name, &construct_uuid.value());
        for input in self.specification.inputs.iter() {
            let value = match evaluated_inputs.inputs.get(&input.name) {
                Some(Ok(value)) => Ok(value.clone()),
                Some(Err(e)) => {
                    return Err((
                        wallets,
                        Diagnostic {
                            span: None,
                            location: None,
                            message: format!("Cannot execute command due to erroring inputs"),
                            level: DiagnosticLevel::Error,
                            documentation: None,
                            example: None,
                            parent_diagnostic: Some(Box::new(e.clone())),
                        },
                    ))
                }
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo(lgalabru): return diagnostic
                },
            }?;
            values.insert(&input.name, value);
        }

        let spec = &self.specification;
        let res = (spec.run_signed_execution)(
            &construct_uuid,
            &self.specification,
            &values,
            &addon_defaults,
            progress_tx,
            wallet_instances,
            wallets,
        )?
        .await;

        for request in action_item_requests.iter_mut() {
            let (status, success) = match &res {
                Ok(_) => (ActionItemStatus::Success(None), true),
                Err((_, diag)) => (ActionItemStatus::Error(diag.clone()), false),
            };
            match request.action_type {
                ActionItemRequestType::ReviewInput(_) => {
                    request.action_status = status.clone();
                }
                ActionItemRequestType::ProvidePublicKey(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                ActionItemRequestType::ProvideSignedTransaction(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                _ => unreachable!(),
            }
        }
        res
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let mut dependencies = vec![];
        for input in self.specification.inputs.iter() {
            match input.typing {
                Type::Object(ref props) => {
                    for prop in props.iter() {
                        let mut blocks_iter = self.block.body.get_blocks(&input.name);
                        while let Some(block) = blocks_iter.next() {
                            let Some(attr) = block.body.get_attribute(&prop.name) else {
                                continue;
                            };
                            collect_constructs_references_from_expression(
                                &attr.value,
                                &mut dependencies,
                            );
                        }
                    }
                }
                _ => {
                    let Some(attr) = self.block.body.get_attribute(&input.name) else {
                        continue;
                    };
                    collect_constructs_references_from_expression(&attr.value, &mut dependencies);
                }
            }
        }
        dependencies
    }
}

pub trait CommandImplementation {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        unreachable!()
    }
    fn run_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        unreachable!()
    }

    fn check_signed_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _wallets_state: WalletsState,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        unreachable!()
    }

    fn run_signed_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _wallets_state: WalletsState,
    ) -> WalletSignFutureResult {
        unreachable!()
    }
}
