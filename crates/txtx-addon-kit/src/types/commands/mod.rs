use execution_conditions::{evaluate_post_conditions, evaluate_pre_conditions};
use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use std::{
    collections::HashMap,
    future::{self, Future},
    hash::Hash,
    pin::Pin,
    thread::sleep,
    time::Duration,
};
use uuid::Uuid;

use hcl_edit::{expr::Expression, structure::Block, Span};
use indexmap::IndexMap;

use crate::{
    constants::{DocumentationKey, RunbookKey, SignerKey},
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    types::{types::RunbookCompleteAdditionalInfo, AuthorizationContext},
};
use crate::{helpers::hcl::get_object_expression_key, types::stores::ValueStore};

use super::{
    cloud_interface::CloudServiceContext,
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemResponse,
        ActionItemResponseType, ActionItemStatus, Actions, BlockEvent, ProvideInputRequest,
        ProvidedInputResponse, ReviewedInputResponse,
    },
    namespace::Namespace,
    signers::{
        consolidate_nested_execution_result, consolidate_signer_activate_future_result,
        consolidate_signer_future_result, return_synchronous, PrepareSignedNestedExecutionResult,
        SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
    },
    stores::ValueMap,
    types::{ObjectDefinition, ObjectProperty, RunbookSupervisionContext, Type, Value},
    ConstructDid, Did, EvaluatableInput, PackageId, WithEvaluatableInputs,
};

mod execution_conditions;
pub use execution_conditions::AssertionResult;
pub use execution_conditions::ASSERTION_TYPE_ID;
pub use execution_conditions::{PostConditionEvaluatableInput, PostConditionEvaluationResult};
pub use execution_conditions::{PreConditionEvaluatableInput, PreConditionEvaluationResult};

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

    pub fn from<S: ToString, T: IntoIterator<Item = (S, Value)>>(default: T) -> Self {
        let mut outputs = HashMap::new();
        for (key, value) in default {
            outputs.insert(key.to_string(), value);
        }
        Self { outputs }
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

    pub fn insert(&mut self, key: &str, value: Value) {
        self.outputs.insert(key.into(), value);
    }

    /// Applies each of the keys/values of `other` onto `self`
    pub fn apply(&mut self, other: &CommandExecutionResult) {
        for (key, value) in other.outputs.iter() {
            self.outputs.insert(key.clone(), value.clone());
        }
    }

    pub fn runbook_complete_additional_info(&self) -> Option<RunbookCompleteAdditionalInfo> {
        self.outputs
            .get(RunbookKey::RunbookCompleteAdditionalInfo.as_ref())
            .and_then(|i| i.as_runbook_complete_additional_info())
    }
}

#[derive(Clone, Debug)]
pub struct DependencyExecutionResultCache {
    cache: HashMap<ConstructDid, Result<CommandExecutionResult, Diagnostic>>,
}
impl DependencyExecutionResultCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    pub fn get(
        &self,
        construct_did: &ConstructDid,
    ) -> Option<&Result<CommandExecutionResult, Diagnostic>> {
        self.cache.get(construct_did)
    }

    pub fn insert(
        &mut self,
        construct_did: ConstructDid,
        result: Result<CommandExecutionResult, Diagnostic>,
    ) {
        self.cache.insert(construct_did, result);
    }

    /// If `self` does not contain `construct_did`, insert `construct_did` with `other_result`.
    /// If `self` contains `construct_did`, apply `other_result` onto the existing value by only inserting
    /// each of the keys of `other_result` into `self`'s results at `construct_did`.
    pub fn merge(
        &mut self,
        construct_did: &ConstructDid,
        other_result: &CommandExecutionResult,
    ) -> Result<(), Diagnostic> {
        match self.cache.get_mut(&construct_did) {
            Some(Ok(result)) => {
                result.apply(&other_result);
            }
            Some(Err(e)) => return Err(e.clone()),
            None => {
                self.cache.insert(construct_did.clone(), Ok(other_result.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CommandInputsEvaluationResult {
    pub inputs: ValueStore,
    pub unevaluated_inputs: UnevaluatedInputsMap,
}

impl CommandInputsEvaluationResult {
    pub fn get_if_not_unevaluated<'a, ReturnType>(
        &'a self,
        key: &str,
        getter: impl Fn(&'a ValueStore, &str) -> Result<ReturnType, Diagnostic>,
    ) -> Result<ReturnType, Diagnostic> {
        self.unevaluated_inputs.check_for_diagnostic(key)?;
        getter(&self.inputs, key)
    }
}

#[derive(Clone, Debug)]
pub struct UnevaluatedInputsMap {
    pub map: IndexMap<String, Option<Diagnostic>>,
}
impl UnevaluatedInputsMap {
    pub fn new() -> Self {
        Self { map: IndexMap::new() }
    }

    pub fn insert(&mut self, key: String, value: Option<Diagnostic>) {
        self.map.insert(key, value);
    }
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn check_for_diagnostic(&self, key: &str) -> Result<(), Diagnostic> {
        match self.map.get(key) {
            Some(diag) => match diag {
                Some(diag) => Err(diag.clone()),
                None => {
                    Err(Diagnostic::error_from_string(format!("input '{}' was not evaluated", key)))
                }
            },
            _ => Ok(()),
        }
    }
    pub fn merge(&mut self, other: &UnevaluatedInputsMap) {
        for (key, value) in other.map.iter() {
            self.map.insert(key.clone(), value.clone());
        }
    }
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
            unevaluated_inputs: UnevaluatedInputsMap::new(),
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
    pub self_referencing: bool,
}
impl EvaluatableInput for CommandInput {
    fn documentation(&self) -> String {
        self.documentation.clone()
    }
    fn optional(&self) -> bool {
        self.optional
    }
    fn typing(&self) -> &Type {
        &self.typing
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl CommandInput {
    pub fn as_object(&self) -> Option<&ObjectDefinition> {
        self.typing.as_object()
    }
    pub fn as_array(&self) -> Option<&Box<Type>> {
        self.typing.as_array()
    }
    pub fn as_action(&self) -> Option<&String> {
        self.typing.as_action()
    }
    pub fn as_map(&self) -> Option<&ObjectDefinition> {
        self.typing.as_map()
    }
    pub fn check_value(&self, value: &Value) -> Result<(), Diagnostic> {
        self.typing.check_value(value).map_err(|e| {
            Diagnostic::error_from_string(format!("error in input '{}': {}", self.name, e.message))
        })
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
    pub prepare_nested_execution: CommandPrepareNestedExecution,
    pub run_execution: CommandExecutionClosure,
    pub check_signed_executability: CommandCheckSignedExecutabilityClosure,
    pub evaluate_pre_conditions: CommandEvaluatePreConditions,
    pub prepare_signed_nested_execution: CommandSignedPrepareNestedExecution,
    pub run_signed_execution: CommandSignedExecutionClosure,
    pub build_background_task: CommandBackgroundTaskExecutionClosure,
    pub implements_cloud_service: bool,
    pub aggregate_nested_execution_results: CommandAggregateNestedExecutionResults,
    pub evaluate_post_conditions: CommandEvaluatePostConditions,
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
                name: DocumentationKey::Description.as_ref().into(),
                documentation: "Allows you to describe and comment steps of your runbook".into(),
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
                self_referencing: false,
            },
            CommandInput {
                name: DocumentationKey::Markdown.as_ref().into(),
                documentation: "Allows you to describe and comment steps of your runbook with in-line markdown".into(),
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
                self_referencing: false,
            },
            CommandInput {
                name: DocumentationKey::MarkdownFilepath.as_ref().into(),
                documentation: "Allows you to describe and comment steps of your runbook with a reference to a markdown file in the filesystem".into(),
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false,
                check_performed: false,
                check_required: false,
                sensitive: false,
                self_referencing: false,
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
                self_referencing: false,
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
                self_referencing: false,
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
                self_referencing: false,
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
                self_referencing: false,
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
                self_referencing: false,
            },
        ]
    }
}

impl Serialize for CommandSpecification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandSpecification", 6)?;
        ser.serialize_field("id", &self.matcher)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("documentation", &self.documentation)?;
        ser.serialize_field("inputs", &self.inputs)?;
        ser.serialize_field("outputs", &self.outputs)?;
        ser.serialize_field("example", &self.example)?;
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
        &AuthorizationContext,
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
        &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult,
>;

pub type CommandAggregateNestedExecutionResults = fn(
    &str,
    &ConstructDid,
    &Vec<(ConstructDid, ValueStore)>,
    &Vec<CommandExecutionResult>,
) -> Result<CommandExecutionResult, Diagnostic>;

pub type CommandSignedExecutionClosure = Box<
    fn(
        &ConstructDid,
        &CommandSpecification,
        &ValueStore,
        &channel::Sender<BlockEvent>,
        &HashMap<ConstructDid, SignerInstance>,
        SignersState,
        &AuthorizationContext,
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
    &AuthorizationContext,
) -> Result<Actions, Diagnostic>;

pub type CommandCheckSignedExecutabilityClosure = fn(
    &ConstructDid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &RunbookSupervisionContext,
    &HashMap<ConstructDid, SignerInstance>,
    SignersState,
    &AuthorizationContext,
) -> SignerActionsFutureResult;

pub type CommandEvaluatePreConditions = fn(
    &ConstructDid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &channel::Sender<BlockEvent>,
    &Uuid,
) -> Result<PreConditionEvaluationResult, Diagnostic>;

pub type CommandEvaluatePostConditions = fn(
    &ConstructDid,
    &str,
    &CommandSpecification,
    &ValueStore,
    &mut CommandExecutionResult,
    &channel::Sender<BlockEvent>,
    &Uuid,
) -> Result<PostConditionEvaluationResult, Diagnostic>;

pub type CommandSignedPrepareNestedExecution = fn(
    &ConstructDid,
    &str,
    &ValueStore,
    &HashMap<ConstructDid, SignerInstance>,
    SignersState,
) -> PrepareSignedNestedExecutionResult;

pub type CommandPrepareNestedExecution =
    fn(&ConstructDid, &str, &ValueStore) -> Result<Vec<(ConstructDid, ValueStore)>, Diagnostic>;

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
    Action(String),
    Prompt,
    Module,
    Addon,
}

impl CommandInstanceType {
    pub fn to_ident(&self) -> &str {
        match self {
            CommandInstanceType::Variable => "variable",
            CommandInstanceType::Output => "output",
            CommandInstanceType::Action(_) => "action",
            CommandInstanceType::Prompt => "prompt",
            CommandInstanceType::Module => "module",
            CommandInstanceType::Addon => "addon",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandInstance {
    pub specification: CommandSpecification,
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
    pub namespace: Namespace,
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

impl WithEvaluatableInputs for CommandInstance {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn block(&self) -> &Block {
        &self.block
    }
    /// Checks the `CommandInstance` HCL Block for an attribute named `input.name`
    fn get_expression_from_input(&self, input_name: &str) -> Option<Expression> {
        visit_optional_untyped_attribute(&input_name, &self.block)
    }

    fn get_blocks_for_map(
        &self,
        input_name: &str,
        input_typing: &Type,
        input_optional: bool,
    ) -> Result<Option<Vec<Block>>, Vec<Diagnostic>> {
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
        if entries.is_empty() {
            if !input_optional {
                return Err(vec![Diagnostic::error_from_string(format!(
                    "command '{}' (type '{}') is missing value for object '{}'",
                    self.name, self.specification.matcher, input_name
                ))]);
            } else {
                return Ok(None);
            }
        }
        Ok(Some(entries))
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
                "command '{}' (type '{}') expected object for input '{}'",
                self.name, self.specification.matcher, input_name
            ))]),
        }
    }

    fn get_expression_from_object_property(
        &self,
        input_name: &str,
        prop: &ObjectProperty,
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
    fn _spec_inputs(&self) -> Vec<Box<dyn EvaluatableInput>> {
        self.specification
            .inputs
            .iter()
            .chain(self.specification.default_inputs.iter())
            .filter_map(|x| {
                if x.self_referencing {
                    None
                } else {
                    Some(Box::new(x.clone()) as Box<dyn EvaluatableInput>)
                }
            })
            .collect::<Vec<_>>()
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

    pub fn get_group(&self) -> String {
        let Some(group) = self.block.body.get_attribute("group") else {
            return format!("{} Review", self.specification.name.to_string());
        };
        group.value.to_string()
    }

    pub fn evaluate_pre_conditions(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> Result<PreConditionEvaluationResult, Diagnostic> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);
        let spec = &self.specification;
        (spec.evaluate_pre_conditions)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            progress_tx,
            background_tasks_uuid,
        )
    }

    pub fn check_executability(
        &mut self,
        construct_did: &ConstructDid,
        nested_evaluation_values: &ValueStore,
        evaluated_inputs: &mut CommandInputsEvaluationResult,
        _signer_instances: &mut HashMap<ConstructDid, SignerInstance>,
        action_item_response: &Option<&Vec<ActionItemResponse>>,
        supervision_context: &RunbookSupervisionContext,
        auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        let mut values = ValueStore::new(
            &format!("{}_inputs", self.specification.matcher),
            &construct_did.value(),
        )
        .with_defaults(&evaluated_inputs.inputs.defaults)
        .append_inputs(&nested_evaluation_values.inputs);

        let mut consolidated_actions = Actions::none();
        match action_item_response {
            Some(responses) => {
                responses.into_iter().for_each(|ActionItemResponse { action_item_id, payload }| {
                    match payload {
                        ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                            input_name,
                            value_checked,
                            ..
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
                                    .insert(SignerKey::SignedTransactionBytes.as_ref(), Value::string(bytes.clone())),
                                None => values.insert(SignerKey::SignedTransactionBytes.as_ref(), Value::null()),
                            }
                        }
                        ActionItemResponseType::ProvideSignedMessage(response) => {
                            values.insert(
                                SignerKey::SignedMessageBytes.as_ref(),
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
                auth_context,
            )?;
            consolidated_actions.append(&mut actions);
        }
        Ok(consolidated_actions)
    }

    pub async fn perform_execution(
        &self,
        construct_did: &ConstructDid,
        nested_evaluation_values: &ValueStore,
        evaluated_inputs: &CommandInputsEvaluationResult,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponse>>,
        progress_tx: &channel::Sender<BlockEvent>,
        auth_ctx: &AuthorizationContext,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .append_inputs(&nested_evaluation_values.inputs);

        let spec = &self.specification;
        let res = (spec.run_execution)(
            &construct_did,
            &self.specification,
            &values,
            progress_tx,
            auth_ctx,
        )?
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
                ActionItemRequestType::SendTransaction(_) => {
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

    pub async fn prepare_signed_nested_execution(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        signers: SignersState,
        signer_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> Result<(SignersState, Vec<(ConstructDid, ValueStore)>), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let spec = &self.specification;
        let future = (spec.prepare_signed_nested_execution)(
            &construct_did,
            &self.name,
            &values,
            signer_instances,
            signers,
        );
        return consolidate_nested_execution_result(future, self.block.span()).await;
    }

    pub fn prepare_nested_execution(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
    ) -> Result<Vec<(ConstructDid, ValueStore)>, Diagnostic> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let spec = &self.specification;

        (spec.prepare_nested_execution)(&construct_did, &self.name, &values)
    }

    pub async fn check_signed_executability(
        &mut self,
        construct_did: &ConstructDid,
        nested_evaluation_values: &ValueStore,
        evaluated_inputs: &CommandInputsEvaluationResult,
        signers: SignersState,
        signer_instances: &mut HashMap<ConstructDid, SignerInstance>,
        action_item_response: &Option<&Vec<ActionItemResponse>>,
        action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &AuthorizationContext,
    ) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .append_inputs(&nested_evaluation_values.inputs);

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
                        ActionItemResponseType::SendTransaction(_) => {
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

        let spec = &self.specification;
        let future = (spec.check_signed_executability)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            &supervision_context,
            signer_instances,
            signers,
            auth_ctx,
        );
        let (signer_state, mut actions) =
            consolidate_signer_future_result(future, self.block.span()).await?;
        consolidated_actions.append(&mut actions);
        consolidated_actions.filter_existing_action_items(action_item_requests);
        Ok((signer_state, consolidated_actions))
    }

    pub async fn perform_signed_execution(
        &self,
        construct_did: &ConstructDid,
        nested_evaluation_values: &ValueStore,
        evaluated_inputs: &CommandInputsEvaluationResult,
        signers: SignersState,
        signer_instances: &HashMap<ConstructDid, SignerInstance>,
        action_item_requests: &mut Vec<&mut ActionItemRequest>,
        _action_item_responses: &Option<&Vec<ActionItemResponse>>,
        progress_tx: &channel::Sender<BlockEvent>,
        auth_context: &AuthorizationContext,
    ) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .append_inputs(&nested_evaluation_values.inputs);

        let spec = &self.specification;
        let future = (spec.run_signed_execution)(
            &construct_did,
            &self.specification,
            &values,
            progress_tx,
            signer_instances,
            signers,
            auth_context,
        );
        let res = consolidate_signer_activate_future_result(future, self.block.span()).await?;

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
                ActionItemRequestType::SendTransaction(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                ActionItemRequestType::ProvideSignedMessage(_) => {
                    if success {
                        request.action_status = status.clone();
                    }
                }
                // idk what this does
                ActionItemRequestType::VerifyThirdPartySignature(_) => {
                    // if success {
                    //     request.action_status = status.clone();
                    // }
                }
                _ => {}
            }
        }
        res
    }

    pub fn build_background_task(
        &self,
        construct_did: &ConstructDid,
        nested_evaluation_values: &ValueStore,
        evaluated_inputs: &CommandInputsEvaluationResult,
        execution_result: &CommandExecutionResult,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
        cloud_svc_context: &CloudServiceContext,
    ) -> CommandExecutionFutureResult {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .with_inputs_from_map(&execution_result.outputs)
            .append_inputs(&nested_evaluation_values.inputs);
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
            &if spec.implements_cloud_service { Some(cloud_svc_context.clone()) } else { None },
        );
        res
    }

    pub fn aggregate_nested_execution_results(
        &self,
        construct_did: &ConstructDid,
        nested_values: &Vec<(ConstructDid, ValueStore)>,
        commands_execution_results: &HashMap<ConstructDid, CommandExecutionResult>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut nested_results = vec![];
        for (nested_construct_did, _) in nested_values {
            let nested_result = commands_execution_results
                .get(nested_construct_did)
                .cloned()
                .unwrap_or_else(|| {
                    return CommandExecutionResult::new();
                });
            nested_results.push(nested_result);
        }

        (self.specification.aggregate_nested_execution_results)(
            &self.name,
            &construct_did,
            &nested_values,
            &nested_results,
        )
    }

    pub fn evaluate_post_conditions(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        command_execution_results: &mut CommandExecutionResult,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> Result<PostConditionEvaluationResult, Diagnostic> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let spec = &self.specification;
        let res = (spec.evaluate_post_conditions)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            command_execution_results,
            progress_tx,
            background_tasks_uuid,
        );

        match res.as_ref() {
            Ok(PostConditionEvaluationResult::Retry(backoff)) => {
                sleep(Duration::from_millis(*backoff as u64));
            }
            _ => {}
        }

        res
    }
}

impl ConstructInstance for CommandInstance {
    fn block(&self) -> &Block {
        &self.block
    }
    fn inputs(&self) -> Vec<Box<dyn EvaluatableInput>> {
        let mut res = self
            .specification
            .inputs
            .iter()
            .chain(&self.specification.default_inputs)
            .map(|input| Box::new(input.clone()) as Box<dyn EvaluatableInput>)
            .collect::<Vec<Box<dyn EvaluatableInput>>>();

        res.push(Box::new(PreConditionEvaluatableInput::new()));
        res.push(Box::new(PostConditionEvaluatableInput::new()));

        res
    }

    fn accepts_arbitrary_inputs(&self) -> bool {
        self.specification.accepts_arbitrary_inputs
    }
}

pub trait ConstructInstance {
    /// The HCL block of the construct
    fn block(&self) -> &Block;
    fn inputs(&self) -> Vec<Box<dyn EvaluatableInput>>;
    fn accepts_arbitrary_inputs(&self) -> bool {
        false
    }

    fn get_expressions_referencing_commands_from_inputs(
        &self,
    ) -> Vec<(Option<Box<dyn EvaluatableInput>>, Expression)> {
        let mut expressions = vec![];
        for input in self.inputs().into_iter() {
            let typing = input.typing().clone();
            typing.get_expressions_referencing_constructs(&self.block(), input, &mut expressions);
        }
        if self.accepts_arbitrary_inputs() {
            for attribute in self.block().body.attributes() {
                let mut references = vec![];
                collect_constructs_references_from_expression(
                    &attribute.value,
                    None,
                    &mut references,
                );
                expressions.append(&mut references);
            }
        }
        expressions
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
        _auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _auth_context: &AuthorizationContext,
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
        _auth_ctx: &AuthorizationContext,
    ) -> SignerActionsFutureResult {
        unimplemented!()
    }

    fn prepare_signed_nested_execution(
        construct_did: &ConstructDid,
        instance_name: &str,
        _values: &ValueStore,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers_state: SignersState,
    ) -> PrepareSignedNestedExecutionResult {
        let signer_state = signers_state
            .get_first_signer()
            .expect(&format!("no signers provided for action '{}'", instance_name));
        return_synchronous((
            signers_state,
            signer_state,
            vec![(
                construct_did.clone(),
                ValueStore::new(&construct_did.to_string(), &construct_did.0),
            )],
        ))
    }

    fn evaluate_pre_conditions(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        values: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> Result<PreConditionEvaluationResult, Diagnostic> {
        evaluate_pre_conditions(
            construct_did,
            instance_name,
            spec,
            values,
            progress_tx,
            background_tasks_uuid,
        )
    }

    fn prepare_nested_execution(
        construct_did: &ConstructDid,
        _instance_name: &str,
        _values: &ValueStore,
    ) -> Result<Vec<(ConstructDid, ValueStore)>, Diagnostic> {
        Ok(vec![(
            construct_did.clone(),
            ValueStore::new(&construct_did.to_string(), &construct_did.0),
        )])
    }

    fn aggregate_nested_execution_results(
        _instance_name: &str,
        _construct_did: &ConstructDid,
        _values: &Vec<(ConstructDid, ValueStore)>,
        nested_results: &Vec<CommandExecutionResult>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();
        for nested_result in nested_results {
            result.outputs.extend(nested_result.outputs.clone());
        }
        Ok(result)
    }

    fn run_signed_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _signers_state: SignersState,
        _auth_context: &AuthorizationContext,
    ) -> SignerSignFutureResult {
        unimplemented!()
    }

    fn build_background_task(
        _construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _outputs: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
        _cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }

    fn evaluate_post_conditions(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        values: &ValueStore,
        execution_results: &mut CommandExecutionResult,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> Result<PostConditionEvaluationResult, Diagnostic> {
        evaluate_post_conditions(
            construct_did,
            instance_name,
            spec,
            values,
            execution_results,
            progress_tx,
            background_tasks_uuid,
        )
    }
}

pub fn add_ctx_to_diag(
    command_type: String,
    matcher: String,
    command_instance_name: String,
    namespace: Namespace,
) -> impl Fn(&Diagnostic) -> Diagnostic {
    let diag_with_command_ctx = move |diag: &Diagnostic| -> Diagnostic {
        let mut diag = diag.clone();
        diag.message = format!(
            "'{}:{}' {} '{}': {}",
            namespace, matcher, command_type, command_instance_name, diag.message
        );
        diag
    };
    return diag_with_command_ctx;
}
pub fn add_ctx_to_embedded_runbook_diag(
    embedded_runbook_instance_name: String,
) -> impl Fn(&Diagnostic) -> Diagnostic {
    let diag_with_command_ctx = move |diag: &Diagnostic| -> Diagnostic {
        let mut diag = diag.clone();
        diag.message =
            format!("embedded runbook '{}': {}", embedded_runbook_instance_name, diag.message);
        diag
    };
    return diag_with_command_ctx;
}
