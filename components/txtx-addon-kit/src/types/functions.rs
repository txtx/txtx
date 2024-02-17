use super::{
    diagnostics::Diagnostic,
    types::{Typing, Value},
};

#[derive(Clone, Debug)]
pub struct FunctionInput {
    pub name: String,
    pub documentation: String,
    pub typing: Vec<Typing>,
}

#[derive(Clone, Debug)]
pub struct FunctionOutput {
    pub documentation: String,
    pub typing: Typing,
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
type FunctionChecker = fn(&FunctionSpecification, &Vec<Typing>) -> Result<Typing, Diagnostic>;

pub trait FunctionImplementation {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic>;
    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic>;
}
