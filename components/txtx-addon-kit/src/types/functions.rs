use super::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};

#[derive(Clone, Debug)]
pub struct FunctionInput {
    pub name: String,
    pub documentation: String,
    pub typing: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct FunctionOutput {
    pub documentation: String,
    pub typing: Type,
}

#[derive(Clone, Debug)]
pub struct FunctionSpecification {
    pub name: String,
    pub documentation: String,
    pub inputs: Vec<FunctionInput>,
    pub output: FunctionOutput,
    pub example: String,
    pub snippet: String,
    pub runner: FunctionRunner,
    pub checker: FunctionChecker,
}

type FunctionRunner = fn(&FunctionSpecification, &Vec<Value>) -> Result<Value, Diagnostic>;
type FunctionChecker = fn(&FunctionSpecification, &Vec<Type>) -> Result<Type, Diagnostic>;

pub trait FunctionImplementation {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic>;
    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic>;
}
