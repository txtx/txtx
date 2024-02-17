use std::collections::HashMap;

use hcl_edit::{expr::Expression, structure::Block};

use crate::helpers::hcl::{
    collect_constructs_references_from_expression, visit_optional_untyped_attribute,
};

use super::{
    diagnostics::Diagnostic,
    types::{ObjectProperty, Typing, Value},
    PackageUuid,
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
    pub inputs: HashMap<CommandInput, Result<Value, Diagnostic>>, // todo(lgalabru): replace Value with EvaluatedExpression
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
    pub typing: Typing,
    pub optional: bool,
    pub interpolable: bool,
}

impl CommandInput {
    pub fn as_object(&self) -> Option<&Vec<ObjectProperty>> {
        match &self.typing {
            Typing::Object(spec) => Some(spec),
            Typing::Primitive(_) => None,
            Typing::Addon(_) => None,
        }
    }
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
    pub runner: CommandRunner,
    pub checker: CommandChecker,
}

type CommandChecker = fn(&CommandSpecification, Vec<Typing>) -> Result<Typing, Diagnostic>;
type CommandRunner = fn(
    &CommandSpecification,
    &HashMap<String, Value>,
) -> Result<CommandExecutionResult, Diagnostic>;

pub trait CommandImplementation {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Result<Typing, Diagnostic>;
    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic>;
}

#[derive(Clone, Debug)]
pub struct CommandInstance {
    pub specification: CommandSpecification,
    pub name: String,
    pub block: Block,
    pub package_uuid: PackageUuid,
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

    pub fn get_expression_from_input(
        &self,
        input: &CommandInput,
    ) -> Result<Option<Expression>, String> {
        let res = match &input.typing {
            Typing::Primitive(_) => visit_optional_untyped_attribute(&input.name, &self.block)
                .map_err(|e| format!("{:?}", e))?,
            Typing::Object(_) => unreachable!(),
            Typing::Addon(_) => unreachable!(),
        };
        match (res, input.optional) {
            (Some(res), _) => Ok(Some(res)),
            (None, true) => Ok(None),
            (None, false) => Err(format!(
                "command '{}' (type '{}') is missing value for field '{}'",
                self.name, self.specification.matcher, input.name
            )),
        }
    }

    pub fn get_expression_from_object_property(
        &self,
        input: &CommandInput,
        prop: &ObjectProperty,
    ) -> Result<Option<Expression>, String> {
        let object = self.block.body.get_blocks(&input.name).next();
        match (object, input.optional) {
            (Some(block), _) => {
                let expr_res = visit_optional_untyped_attribute(&prop.name, &block)
                    .map_err(|e| format!("{:?}", e))?;
                match (expr_res, prop.optional) {
                    (Some(expression), _) => Ok(Some(expression)),
                    (None, true) => Ok(None),
                    (None, false) => Err(format!(
                        "command '{}' (type '{}') is missing property '{}' for object '{}'",
                        self.name, self.specification.matcher, prop.name, input.name
                    )),
                }
            }
            (None, true) => Ok(None),
            (None, false) => Err(format!(
                "command '{}' (type '{}') is missing object '{}'",
                self.name, self.specification.matcher, input.name
            )),
        }
    }

    pub fn perform_execution(
        &self,
        evaluated_inputs: &CommandInputsEvaluationResult,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut values = HashMap::new();
        for input in self.specification.inputs.iter() {
            let value = match evaluated_inputs.inputs.get(input) {
                Some(Ok(value)) => Ok(value.clone()),
                Some(Err(e)) => Err(e.clone()),
                None => continue,
            }?;
            values.insert(input.name.clone(), value);
        }
        (self.specification.runner)(&self.specification, &values)
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let mut dependencies = vec![];
        for input in self.specification.inputs.iter() {
            let Some(attr) = self.block.body.get_attribute(&input.name) else {
                continue;
            };
            collect_constructs_references_from_expression(&attr.value, &mut dependencies);
        }
        dependencies
    }
}
