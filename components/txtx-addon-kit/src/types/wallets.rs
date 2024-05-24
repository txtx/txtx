use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{mpsc::Sender, Arc, Mutex},
};

use hcl_edit::{expr::Expression, structure::Block};
use rust_fsm::StateMachine;

#[cfg(not(feature = "wasm"))]
use tokio::runtime::Builder as RuntimeBuilder;

use uuid::Uuid;

use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};

use super::{
    commands::{
        CommandExecutionResult, CommandExecutionStatus, CommandInput,
        CommandInputsEvaluationResult, CommandInstanceStateMachine, CommandOutput, EvalEvent,
    },
    diagnostics::{Diagnostic, DiagnosticLevel},
    types::{ObjectProperty, Type, Value},
    ConstructUuid, PackageUuid,
};

pub type WalletChecker = fn(&WalletSpecification, Vec<Type>) -> Result<Type, Diagnostic>;
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WalletRunner {
    Async(WalletRunnerAsync),
    Sync(WalletRunnerSync),
}

type WalletRunnerSync = fn(
    &WalletSpecification,
    &HashMap<String, Value>,
    &AddonDefaults,
) -> Result<CommandExecutionResult, Diagnostic>;

type WalletRunnerAsync = Box<
    fn(
        &WalletSpecification,
        &HashMap<String, Value>,
        &AddonDefaults,
    ) -> Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>>>>,
>;

pub type PublicKeySetter =
    fn(&WalletSpecification, &mut CommandInputsEvaluationResult, String, String);

#[derive(Debug, Clone)]
pub struct WalletSpecification {
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
    pub signer: WalletRunner,
    pub public_key_setter: PublicKeySetter,
    pub checker: WalletChecker,
}

#[derive(Debug, Clone)]
pub struct WalletInstance {
    pub specification: WalletSpecification,
    pub state: Arc<Mutex<StateMachine<CommandInstanceStateMachine>>>,
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
        match &self.specification.signer {
            WalletRunner::Async(async_runner) => {
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
            WalletRunner::Sync(sync_runner) => Ok(CommandExecutionStatus::Complete((sync_runner)(
                &self.specification,
                &values,
                &addon_defaults,
            ))),
        }
    }
}

pub trait WalletImplementationAsync {
    fn check(_ctx: &WalletSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic>;
    fn sign(
        _ctx: &WalletSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>>>>;
    fn set_public_keys(
        _ctx: &WalletSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    );
}
