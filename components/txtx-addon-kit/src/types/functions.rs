use super::typing::{Typing, Value};

#[derive(Clone)]
pub struct FunctionInput {
    pub name: String,
    pub documentation: String,
    pub typing: Typing,
}

#[derive(Clone)]
pub struct FunctionOutput {
    pub documentation: String,
    pub typing: Typing,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct TypingDeclaration {
    pub name: String,
    pub documentation: String,
    pub check: TypingChecker,
}

type FunctionRunner = fn(&FunctionSpecification, Vec<Value>) -> Value;
type FunctionChecker = fn(&FunctionSpecification, Vec<Typing>) -> Typing;

pub trait FunctionImplementation {
    fn check(ctx: &FunctionSpecification, args: Vec<Typing>) -> Typing;
    fn run(ctx: &FunctionSpecification, args: Vec<Value>) -> Value;
}

type TypingChecker = fn(&TypingDeclaration, Vec<Typing>) -> (bool, Option<Typing>);
pub trait TypingImplementation {
    fn check(ctx: &TypingDeclaration, args: Vec<Typing>) -> Typing;
}
