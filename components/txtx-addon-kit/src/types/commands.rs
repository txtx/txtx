use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use std::collections::HashMap;

use hcl_edit::{expr::Expression, structure::Block};

use crate::helpers::hcl::{
    collect_constructs_references_from_expression, visit_optional_untyped_attribute,
};

use super::{
    diagnostics::Diagnostic,
    types::{ObjectProperty, Type, Value},
    PackageUuid,
};

#[derive(Clone, Debug)]
pub struct CommandExecutionResult {
    pub outputs: HashMap<String, Value>,
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

#[derive(Clone, Debug)]
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

type CommandChecker = fn(&CommandSpecification, Vec<Type>) -> Result<Type, Diagnostic>;
type CommandRunner = fn(
    &CommandSpecification,
    &HashMap<String, Value>,
) -> Result<CommandExecutionResult, Diagnostic>;

pub trait CommandImplementation {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic>;
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
    pub input_evaluation_result: HashMap<CommandInput, Value>,
}

impl Serialize for CommandInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("CommandInstance", 3)?;
        ser.serialize_field("specification", &self.specification)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("packageUuid", &self.package_uuid)?;
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
            Type::Primitive(_) => visit_optional_untyped_attribute(&input.name, &self.block)
                .map_err(|e| format!("{:?}", e))?,
            Type::Object(_) => unreachable!(),
            Type::Addon(_) => unreachable!(),
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
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo(lgalabru): return diagnostic
                },
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
