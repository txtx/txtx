use std::collections::HashMap;

use hcl_edit::{expr::Expression, structure::Block, visit::visit_element};

use crate::helpers::{
    fs::FileLocation,
    hcl::{collect_constructs_references_from_expression, visit_optional_untyped_attribute},
};

use super::{
    diagnostics::Diagnostic,
    typing::{Typing, Value},
    ConstructUuid,
};

#[derive(Clone, Debug)]
pub struct CommandExecutionResult {
    pub outputs: HashMap<String, Value>,
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
    pub inputs: HashMap<String, Value>,
}

impl CommandInputsEvaluationResult {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandInput {
    pub name: String,
    pub documentation: String,
    pub typing: Typing,
    pub optional: bool,
    pub interpolable: bool,
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub name: String,
    pub documentation: String,
    pub typing: Typing,
}

#[derive(Clone, Debug)]
pub struct CommandSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub accepts_arbitrary_inputs: bool,
    pub create_output_for_each_input: bool,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub runner: ConstructRunner,
    pub checker: ConstructChecker,
}

type ConstructRunner = fn(&CommandSpecification, Vec<Value>) -> Value;
type ConstructChecker = fn(&CommandSpecification, Vec<Typing>) -> Typing;

pub trait CommandImplementation {
    fn check(ctx: &CommandSpecification, args: Vec<Typing>) -> Typing;
    fn run(ctx: &CommandSpecification, args: Vec<Value>) -> Value;
}

#[derive(Clone, Debug)]
pub struct CommandInstance {
    pub specification: CommandSpecification,
    pub name: String,
    pub block: Block,
}

impl CommandInstance {
    pub fn check_inputs(&self) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let has_errors = false;

        for input in self.specification.inputs.iter() {}

        if has_errors {
            Err(diagnostics)
        } else {
            Ok(diagnostics)
        }
    }

    pub fn get_references_expressions_from_inputs(&self) -> Result<Vec<Expression>, String> {
        let mut expressions = vec![];
        for input in self.specification.inputs.iter() {
            let res = visit_optional_untyped_attribute(&input.name, &self.block)
                .map_err(|e| format!("{:?}", e))?;
            if let Some(expr) = res {
                let mut references = vec![];
                collect_constructs_references_from_expression(&expr, &mut references);
                expressions.append(&mut references);
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

    pub fn perform_inputs_evaluation(
        &self,
        dependencies_execution_results: &HashMap<ConstructUuid, &CommandExecutionResult>,
    ) -> CommandInputsEvaluationResult {
        let mut results = CommandInputsEvaluationResult::new();
        println!(
            "Command {}:{} will evaluate its inputs using the executions results from {:?}",
            self.specification.name, self.name, dependencies_execution_results
        );
        results
    }

    pub fn perform_execution(
        &self,
        evaluated_inputs: &CommandInputsEvaluationResult,
    ) -> CommandExecutionResult {
        let mut results = CommandExecutionResult::new();
        println!(
            "Command {}:{} will perform its execution using its evaluated inputs from {:?}",
            self.specification.name, self.name, evaluated_inputs
        );
        (self.specification.runner)(&self.specification, vec![]);
        results
    }
}
