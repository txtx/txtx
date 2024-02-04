#[derive(Clone)]
pub enum Value {
    String(String),
    Number(i64),
    Bool(bool),
}

#[derive(Clone)]
pub enum Typing {
    String,
    Number,
    Bool,
}

#[derive(Clone)]
pub struct NativeFunctionInput {
    pub name: String,
    pub documentation: String,
    pub typing: Typing,
}

#[derive(Clone)]
pub struct NativeFunctionOutput {
    pub documentation: String,
    pub typing: Typing,
}

#[derive(Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub documentation: String,
    pub inputs: Vec<NativeFunctionInput>,
    pub output: NativeFunctionOutput,
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

type FunctionRunner = fn(&FunctionDeclaration, Vec<Value>) -> Value;
type FunctionChecker = fn(&FunctionDeclaration, Vec<Typing>) -> Typing;

pub trait FunctionImplementation {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing;
    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value;
}

type TypingChecker = fn(&TypingDeclaration, Vec<Typing>) -> (bool, Option<Typing>);
pub trait TypingImplementation {
    fn check(ctx: &TypingDeclaration, args: Vec<Typing>) -> Typing;
}
