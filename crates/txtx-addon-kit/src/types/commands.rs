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
use uuid::Uuid;

use hcl_edit::{expr::Expression, structure::Block, Span};
use indexmap::IndexMap;

use crate::types::stores::ValueStore;
use crate::{
    constants::{SIGNED_MESSAGE_BYTES, SIGNED_TRANSACTION_BYTES},
    helpers::hcl::{
        collect_constructs_references_from_expression, get_object_expression_key,
        visit_optional_untyped_attribute,
    },
};

use super::{
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemResponse,
        ActionItemResponseType, ActionItemStatus, Actions, BlockEvent, ProvideInputRequest,
        ProvidedInputResponse, ReviewedInputResponse,
    },
    signers::{
        consolidate_signer_activate_future_result, consolidate_signer_future_result,
        SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
    },
    stores::ValueMap,
    types::{ObjectProperty, RunbookSupervisionContext, Type, Value},
    ConstructDid, Did, PackageId,
};

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
        Self { outputs: HashMap::new() }
    }

    pub fn append(&mut self, other: &mut CommandExecutionResult) {
        for (key, value) in other.outputs.drain() {
            self.outputs.insert(key, value);
        }
    }

    pub fn from_value_store(store: &ValueStore) -> Self {
        let mut outputs = HashMap::new();
        for (key, value) in store.iter() {
            outputs.insert(key.clone(), value.clone());
        }
        Self { outputs }
    }
}

#[derive(Clone, Debug)]
pub struct CommandInputsEvaluationResult {
    pub inputs: ValueStore,
    pub unevaluated_inputs: IndexMap<String, Option<Diagnostic>>,
}

impl Serialize for CommandInputsEvaluationResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inputs.len()))?;
        for (k, v) in self.inputs.iter() {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

impl CommandInputsEvaluationResult {
    pub fn new(name: &str, defaults: &ValueMap) -> Self {
        Self {
            inputs: ValueStore::new(&format!("{name}_inputs"), &Did::zero())
                .with_defaults(defaults),
            unevaluated_inputs: IndexMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: Value) {
        self.inputs.insert(key, value);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CommandInput {
    pub name: String,
    pub documentation: String,
    pub typing: Type,
    pub optional: bool,
    pub tainting: bool,
    pub check_required: bool,
    pub check_performed: bool,
    pub sensitive: bool,
    pub internal: bool,
}

impl CommandInput {
    pub fn as_object(&self) -> Option<&Vec<ObjectProperty>> {
        match &self.typing {
            Type::Object(spec) => Some(spec),
            Type::Addon(_) => None,
            Type::Array(_) => None,
            Type::Bool => None,
            Type::Null => None,
            Type::Integer => None,
            Type::Float => None,
            Type::String => None,
            Type::Buffer => None,
        }
    }
    pub fn as_array(&self) -> Option<&Box<Type>> {
        match &self.typing {
            Type::Object(_) => None,
            Type::Addon(_) => None,
            Type::Array(array) => Some(array),
            Type::Bool => None,
            Type::Null => None,
            Type::Integer => None,
            Type::Float => None,
            Type::String => None,
            Type::Buffer => None,
        }
    }
    pub fn as_action(&self) -> Option<&String> {
        match &self.typing {
            Type::Object(_) => None,
            Type::Addon(addon) => Some(addon),
            Type::Array(_) => None,
            Type::Bool => None,
            Type::Null => None,
            Type::Integer => None,
            Type::Float => None,
            Type::String => None,
            Type::Buffer => None,
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

    pub fn action_name(&self) -> String {
        match &self {
            &CommandId::Action(id) => format!("{id}"),
        }
    }
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
    pub create_critical_output: Option<String>,
    pub update_addon_defaults: bool,
    pub implements_signing_capability: bool,
    pub implements_background_task_capability: bool,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub inputs_post_processing_closure: InputsPostProcessingClosure,
    pub check_instantiability: InstantiabilityChecker,
    pub check_executability: CommandCheckExecutabilityClosure,
    pub run_execution: CommandExecutionClosure,
    pub check_signed_executability: CommandCheckSignedExecutabilityClosure,
    pub run_signed_execution: CommandSignedExecutionClosure,
    pub build_background_task: CommandBackgroundTaskExecutionClosure,
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
                tainting: true,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
            },
            CommandInput {
                name: "labels".into(),
                documentation: "Allows you to label steps of your runbook".into(),
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
            },
            CommandInput {
                name: "environments".into(),
                documentation: "Only enable command for given environments (default: all)".into(),
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
            },
            CommandInput {
                name: "sensitive".into(),
                documentation: "Never include value in logs".into(),
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: true,
                check_performed: false,
                check_required: false,
                sensitive: false,
            },
            CommandInput {
                name: "group".into(),
                documentation: "Name used for grouping commands together".into(),
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: true,
                check_performed: false,
                check_required: false,
                sensitive: false,
            },
            CommandInput {
                name: "depends_on".into(),
                documentation: "Name used for grouping commands together".into(),
                typing: Type::array(Type::string()),
                optional: true,
                tainting: false,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
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

pub type InputsPostProcessingClosure =
    fn(&CommandSpecification, CommandInputsEvaluationResult) -> InputsPostProcessingFutureResult;
pub type InputsPostProcessingFutureResult = Result<InputsPostProcessingFuture, Diagnostic>;
pub type InputsPostProcessingFuture =
    Pin<Box<dyn Future<Output = Result<CommandInputsEvaluationResult, Diagnostic>> + Send>>;

pub type CommandExecutionFutureResult = Result<CommandExecutionFuture, Diagnostic>;
pub type CommandExecutionFuture =
    Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>> + Send>>;

pub type CommandExecutionClosure = Box<
    fn(
        &ConstructDid,
        &CommandSpecification,
        &ValueStore,
        &channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult,
>;

pub type CommandBackgroundTaskExecutionClosure = Box<
    fn(
        &ConstructDid,
        &CommandSpecification,
        &ValueStore,
        &ValueStore,
        &channel::Sender<BlockEvent>,
        &Uuid,
        &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult,
>;

pub type CommandSignedExecutionClosure = Box<
    fn(
        &ConstructDid,
        &CommandSpecification,
        &ValueStore,
        &channel::Sender<BlockEvent>,
        &HashMap<ConstructDid, SignerInstance>,
        SignersState,
    ) -> SignerSignFutureResult,
>;

type CommandRouter =
    fn(&String, &String, &Vec<PreCommandSpecification>) -> Result<Vec<String>, Diagnostic>;

pub type CommandCheckExecutabilityClosure = fn(
    &ConstructDid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &RunbookSupervisionContext,
) -> Result<Actions, Diagnostic>;

pub type CommandCheckSignedExecutabilityClosure = fn(
    &ConstructDid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &RunbookSupervisionContext,
    &HashMap<ConstructDid, SignerInstance>,
    SignersState,
) -> SignerActionsFutureResult;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandInstanceType {
    Variable,
    Output,
    Action,
    Prompt,
    Module,
    Addon,
}

#[derive(Debug, Clone)]
pub struct CommandInstance {
    pub specification: CommandSpecification,
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
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
        ser.serialize_field("packageUuid", &self.package_id.did())?;
        ser.serialize_field("namespace", &self.namespace)?;
        ser.serialize_field("typing", &self.typing)?;
        ser.end()
    }
}

impl CommandInstance {
    pub async fn post_process_inputs_evaluations(
        &self,
        inputs_evaluation: CommandInputsEvaluationResult,
    ) -> Result<CommandInputsEvaluationResult, Diagnostic> {
        let spec = &self.specification;
        let future = (self.specification.inputs_post_processing_closure)(spec, inputs_evaluation)?;
        let res = future.await?;
        Ok(res)
    }

    pub fn get_expressions_referencing_commands_from_inputs(
        &self,
    ) -> Result<Vec<(Option<&CommandInput>, Expression)>, String> {
        let mut expressions = vec![];
        for input in self.specification.inputs.iter() {
            match input.typing {
                Type::Object(ref props) => {
                    let res = visit_optional_untyped_attribute(&input.name, &self.block)
                        .map_err(|e| format!("{:?}", e))?;
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
                        let mut blocks_iter = self.block.body.get_blocks(&input.name);
                        while let Some(block) = blocks_iter.next() {
                            let res = visit_optional_untyped_attribute(&prop.name, &block)
                                .map_err(|e| format!("{:?}", e))?;
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
                    let res = visit_optional_untyped_attribute(&input.name, &self.block)
                        .map_err(|e| format!("{:?}", e))?;
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
        if self.specification.accepts_arbitrary_inputs {
            for attribute in self.block.body.attributes() {
                let mut references = vec![];
                collect_constructs_references_from_expression(
                    &attribute.value,
                    None,
                    &mut references,
                );
                expressions.append(&mut references);
            }
        }
        Ok(expressions)
    }

    /// Checks the `CommandInstance` HCL Block for an attribute named `input.name`
    pub fn get_expression_from_input(
        &self,
        input: &CommandInput,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        let res = visit_optional_untyped_attribute(&input.name, &self.block)?;
        match (res, input.optional) {
            (Some(res), _) => Ok(Some(res)),
            (None, true) => Ok(None),
            (None, false) => Err(vec![Diagnostic::error_from_string(format!(
                "command '{}' (type '{}') is missing value for field '{}'",
                self.name, self.specification.matcher, input.name
            ))]),
        }
    }

    pub fn get_group(&self) -> String {
        let Some(group) = self.block.body.get_attribute("group") else {
            return format!("{} Review", self.specification.name.to_string());
        };
        group.value.to_string()
    }

    pub fn get_expression_from_object(
        &self,
        input: &CommandInput,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        let object = match &input.typing {
            Type::Object(_) => visit_optional_untyped_attribute(&input.name, &self.block)?,
            _ => {
                unreachable!()
            }
        };
        match (object, input.optional) {
            (Some(expr), _) => Ok(Some(expr)),
            (None, true) => Ok(None),
            (None, false) => Err(vec![Diagnostic::error_from_string(format!(
                "command '{}' (type '{}') is missing value for object '{}'",
                self.name, self.specification.matcher, input.name
            ))]),
        }
    }

    pub fn get_expression_from_object_property(
        &self,
        input: &CommandInput,
        prop: &ObjectProperty,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        let expr = visit_optional_untyped_attribute(&input.name, &self.block)?;
        match (expr, input.optional) {
            (Some(expr), _) => {
                let object_expr = expr.as_object().unwrap();
                let expr_res = get_object_expression_key(object_expr, &prop.name);
                match (expr_res, prop.optional) {
                    (Some(expression), _) => Ok(Some(expression.expr().clone())),
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
        construct_did: &ConstructDid,
        evaluated_inputs: &mut CommandInputsEvaluationResult,
        _signer_instances: &mut HashMap<ConstructDid, SignerInstance>,
        action_item_response: &Option<&Vec<ActionItemResponse>>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        let mut values = ValueStore::new(
            &format!("{}_inputs", self.specification.matcher),
            &construct_did.value(),
        )
        .with_defaults(&evaluated_inputs.inputs.defaults);

        let mut consolidated_actions = Actions::none();
        match action_item_response {
            Some(responses) => {
                responses.into_iter().for_each(|ActionItemResponse { action_item_id, payload }| {
                    match payload {
                        ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                            input_name,
                            value_checked,
                        }) => {
                            for input in self.specification.inputs.iter_mut() {
                                if &input.name == input_name {
                                    input.check_performed = value_checked.clone();
                                    break;
                                }
                            }
                        }
                        ActionItemResponseType::ProvideInput(ProvidedInputResponse {
                            input_name,
                            updated_value,
                        }) => {
                            evaluated_inputs.inputs.insert(&input_name, updated_value.clone());

                            let action_item_update =
                                ActionItemRequestUpdate::from_id(&action_item_id)
                                    .set_type(ActionItemRequestType::ProvideInput(
                                        ProvideInputRequest {
                                            default_value: Some(updated_value.clone()),
                                            input_name: input_name.clone(),
                                            typing: updated_value.get_type(),
                                        },
                                    ))
                                    .set_status(ActionItemStatus::Success(None));
                            consolidated_actions.push_action_item_update(action_item_update);

                            for input in self.specification.inputs.iter_mut() {
                                if &input.name == input_name {
                                    input.check_performed = true;
                                    break;
                                }
                            }
                        }
                        ActionItemResponseType::ProvideSignedTransaction(response) => {
                            match &response.signed_transaction_bytes {
                                Some(bytes) => values
                                    .insert(SIGNED_TRANSACTION_BYTES, Value::string(bytes.clone())),
                                None => values.insert(SIGNED_TRANSACTION_BYTES, Value::null()),
                            }
                        }
                        ActionItemResponseType::ProvideSignedMessage(response) => {
                            values.insert(
                                SIGNED_MESSAGE_BYTES,
                                Value::string(response.signed_message_bytes.clone()),
                            );
                        }
                        _ => {}
                    }
                })
            }
            None => {}
        }
        let values = values
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .check(&self.name, &self.specification.inputs)?;

        let spec = &self.specification;
        if spec.matcher != "output" {
            let mut actions = (spec.check_executability)(
                &construct_did,
                &self.name,
                &spec,
                &values,
                &supervision_context,
            )?;
            consolidated_actions.append(&mut actions);
        }
        Ok(consolidated_actions)
    }

    pub async fn perform_execution(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponse>>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let spec = &self.specification;
        let res = (spec.run_execution)(&construct_did, &self.specification, &values, progress_tx)?
            .await
            .map_err(|e| e.set_span_range(self.block.span()));

        for request in action_item_requests.iter_mut() {
            let (status, success) = match &res {
                Ok(_) => (ActionItemStatus::Success(None), true),
                Err(diag) => (ActionItemStatus::Error(diag.clone()), false),
            };
            match request.action_type {
                ActionItemRequestType::ReviewInput(_) => {
                    request.action_status = status.clone();
                }
                ActionItemRequestType::ProvideInput(_) => {
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
                ActionItemRequestType::ProvideSignedMessage(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                _ => unreachable!(),
            }
        }
        res
    }

    pub async fn check_signed_executability(
        &mut self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        signers: SignersState,
        signer_instances: &mut HashMap<ConstructDid, SignerInstance>,
        action_item_response: &Option<&Vec<ActionItemResponse>>,
        action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        // TODO
        let mut consolidated_actions = Actions::none();
        match action_item_response {
            Some(responses) => {
                responses.into_iter().for_each(|ActionItemResponse { action_item_id, payload }| {
                    match payload {
                        ActionItemResponseType::ReviewInput(update) => {
                            // This is a shortcut and should be mutated somewhere else
                            for input in self.specification.inputs.iter_mut() {
                                if input.name == update.input_name {
                                    input.check_performed = true;
                                    break;
                                }
                            }
                        }
                        ActionItemResponseType::ProvideInput(update) => {
                            let action_item_update =
                                ActionItemRequestUpdate::from_id(&action_item_id)
                                    .set_type(ActionItemRequestType::ProvideInput(
                                        ProvideInputRequest {
                                            default_value: Some(update.updated_value.clone()),
                                            input_name: update.input_name.clone(),
                                            typing: update.updated_value.get_type(),
                                        },
                                    ))
                                    .set_status(ActionItemStatus::Success(None));
                            consolidated_actions.push_action_item_update(action_item_update);
                        }
                        ActionItemResponseType::ProvideSignedTransaction(_) => {
                            let action_item_update =
                                ActionItemRequestUpdate::from_id(&action_item_id)
                                    .set_status(ActionItemStatus::Success(None));
                            consolidated_actions.push_action_item_update(action_item_update);
                        }
                        ActionItemResponseType::ProvideSignedMessage(_response) => {
                            let action_item_update =
                                ActionItemRequestUpdate::from_id(&action_item_id)
                                    .set_status(ActionItemStatus::Success(None));
                            consolidated_actions.push_action_item_update(action_item_update);
                        }
                        _ => {}
                    }
                })
            }
            None => {}
        }
        println!("values before checking signed executability {:?}", values);
        let spec = &self.specification;
        let future = (spec.check_signed_executability)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            &supervision_context,
            signer_instances,
            signers,
        );
        let res = consolidate_signer_future_result(future)
            .await?
            .map_err(|(state, diag)| (state, diag.set_span_range(self.block.span())));
        let (signer_state, mut actions) = res?;
        consolidated_actions.append(&mut actions);
        consolidated_actions.filter_existing_action_items(action_item_requests);
        Ok((signer_state, consolidated_actions))
    }

    pub async fn perform_signed_execution(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        signers: SignersState,
        signer_instances: &HashMap<ConstructDid, SignerInstance>,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponse>>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let spec = &self.specification;
        let future = (spec.run_signed_execution)(
            &construct_did,
            &self.specification,
            &values,
            progress_tx,
            signer_instances,
            signers,
        );
        let res = consolidate_signer_activate_future_result(future)
            .await?
            .map_err(|(state, diag)| (state, diag.set_span_range(self.block.span())));

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
                ActionItemRequestType::ProvideSignedMessage(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                _ => {}
            }
        }
        res
    }

    pub fn build_background_task(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        execution_result: &CommandExecutionResult,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .with_inputs_from_map(&execution_result.outputs);
        let outputs = ValueStore::new(&self.name, &construct_did.value())
            .with_inputs_from_map(&execution_result.outputs);

        let spec = &self.specification;
        let res = (spec.build_background_task)(
            &construct_did,
            &self.specification,
            &values,
            &outputs,
            progress_tx,
            background_tasks_uuid,
            supervision_context,
        );
        res
    }

    pub fn collect_dependencies(&self) -> Vec<(Option<&CommandInput>, Expression)> {
        let mut dependencies = vec![];

        for input in self.specification.inputs.iter().chain(&self.specification.default_inputs) {
            match input.typing {
                Type::Object(ref props) => {
                    if let Some(attr) = self.block.body.get_attribute(&input.name) {
                        collect_constructs_references_from_expression(
                            &attr.value,
                            Some(input),
                            &mut dependencies,
                        );
                    } else {
                        for prop in props.iter() {
                            let mut blocks_iter = self.block.body.get_blocks(&input.name);
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
                    let Some(attr) = self.block.body.get_attribute(&input.name) else {
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
        dependencies
    }
}

pub trait CommandImplementation {
    fn post_process_evaluated_inputs(
        _ctx: &CommandSpecification,
        inputs: CommandInputsEvaluationResult,
    ) -> InputsPostProcessingFutureResult {
        let future = async move { Ok(inputs) };
        Ok(Box::pin(future))
    }

    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
    }
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }

    fn check_signed_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _signers_state: SignersState,
    ) -> SignerActionsFutureResult {
        unimplemented!()
    }

    fn run_signed_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _signers_state: SignersState,
    ) -> SignerSignFutureResult {
        unimplemented!()
    }

    fn build_background_task(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _outputs: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }
}
