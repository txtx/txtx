use rust_fsm::{state_machine, StateMachine};
use serde::{
    ser::{Error, SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    pin::Pin,
    sync::{mpsc::Sender, Arc, Mutex},
};
#[cfg(not(feature = "wasm"))]
use tokio::runtime::Builder as RuntimeBuilder;
use uuid::Uuid;

use hcl_edit::{expr::Expression, structure::Block};

use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};

use super::{
    diagnostics::{Diagnostic, DiagnosticLevel},
    types::{ObjectProperty, Type, TypeSpecification, Value},
    ConstructUuid, PackageUuid,
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
        Self {
            outputs: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandInputsEvaluationResult {
    pub inputs: HashMap<CommandInput, Result<Value, Diagnostic>>, // todo(lgalabru): replace Value with EvaluatedExpression
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
            map.serialize_entry(&k.name, &value)?;
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

    pub fn insert(&mut self, command_input: CommandInput, value: Result<Value, Diagnostic>) {
        self.inputs.insert(command_input, value);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CommandInput {
    pub name: String,
    pub documentation: String,
    pub typing: Type,
    pub optional: bool,
    pub interpolable: bool,
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
    Prompt(String),
}

impl CommandId {
    pub fn to_string(&self) -> String {
        match &self {
            &CommandId::Action(id) => format!("action::{id}"),
            &CommandId::Prompt(id) => format!("prompt::{id}"),
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
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub runner: CommandRunner,
    pub checker: CommandChecker,
    pub user_input_parser: CommandParser,
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
            },
            CommandInput {
                name: "labels".into(),
                documentation: "Allows you to label steps of your runbook".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: true,
            },
            CommandInput {
                name: "environments".into(),
                documentation: "Only enable command for given environments (default: all)".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: false,
            },
            CommandInput {
                name: "redacted".into(),
                documentation: "...".into(),
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: false,
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

pub type CommandChecker = fn(&CommandSpecification, Vec<Type>) -> Result<Type, Diagnostic>;
// type CommandRunner = Box<
//     fn(
//         &CommandSpecification,
//         &HashMap<String, Value>,
//     ) -> Result<CommandExecutionResult, Diagnostic>,
// >;
type CommandParser = fn(&CommandSpecification, &mut CommandInputsEvaluationResult, String, String);
type CommandRouter =
    fn(&String, &String, &Vec<PreCommandSpecification>) -> Result<Vec<String>, Diagnostic>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CommandRunner {
    Async(CommandRunnerAsync),
    Sync(CommandRunnerSync),
}

type CommandRunnerSync = fn(
    &CommandSpecification,
    &HashMap<String, Value>,
    &AddonDefaults,
) -> Result<CommandExecutionResult, Diagnostic>;

type CommandRunnerAsync = Box<
    fn(
        &CommandSpecification,
        &HashMap<String, Value>,
        &AddonDefaults,
    ) -> Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>>>>,
>;

pub trait CommandImplementationAsync {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic>;
    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>>>>;
    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    );
}
pub trait CommandImplementation {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic>;
    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic>;
    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    );
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
    pub state: Arc<Mutex<StateMachine<CommandInstanceStateMachine>>>,
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
        let state_machine = self.state.lock().map_err(S::Error::custom)?;
        let state = state_machine.state();
        ser.serialize_field("state", &state)?;
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

    pub fn perform_execution(
        &self,
        evaluated_inputs: &CommandInputsEvaluationResult,
        runbook_uuid: Uuid,
        construct_uuid: ConstructUuid,
        eval_tx: Sender<EvalEvent>,
        addon_defaults: AddonDefaults,
    ) -> Result<CommandExecutionStatus, Diagnostic> {
        // todo: I don't think this one needs to be a result
        let mut values = HashMap::new();
        for input in self.specification.inputs.iter() {
            let value = match evaluated_inputs.inputs.get(input) {
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
            values.insert(input.name.clone(), value);
        }
        match &self.specification.runner {
            CommandRunner::Async(async_runner) => {
                #[cfg(not(feature = "wasm"))]
                {
                    let spec = self.specification.clone();
                    let async_runner_moved = async_runner.clone();
                    let _ = std::thread::spawn(move || {
                        let runtime = RuntimeBuilder::new_current_thread()
                            .enable_time()
                            .enable_io()
                            .build()
                            .unwrap();
                        let result =
                            runtime.block_on((async_runner_moved)(&spec, &values, &addon_defaults));
                        eval_tx.send(EvalEvent::AsyncRequestComplete {
                            runbook_uuid,
                            result: Ok(CommandExecutionStatus::Complete(result)),
                            construct_uuid,
                        })
                    });
                    Ok(CommandExecutionStatus::NeedsAsyncRequest)
                }
                #[cfg(feature = "wasm")]
                panic!("async commands are not enabled for wasm")
            }
            CommandRunner::Sync(sync_runner) => Ok(CommandExecutionStatus::Complete(
                (sync_runner)(&self.specification, &values, &addon_defaults),
            )),
        }
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

    pub fn update_input_evaluation_results_from_user_input(
        self,
        inputs: &mut CommandInputsEvaluationResult,
        input_name: String,
        value: String,
    ) {
        (self.specification.user_input_parser)(&self.specification, inputs, input_name, value);
    }
}

pub enum EvalEvent {
    AsyncRequestComplete {
        runbook_uuid: Uuid,
        result: Result<CommandExecutionStatus, Diagnostic>,
        construct_uuid: ConstructUuid,
    },
}
