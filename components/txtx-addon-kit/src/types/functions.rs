#[derive(Clone)]
pub enum Value {
    String(String),
    Number(i64),
    Bool(bool),
}

#[derive(Clone)]
pub enum TypeSignature {
    String,
    Number,
    Bool,
}

#[derive(Clone)]
pub struct NativeFunctionInput {
    pub name: String,
    pub documentation: String,
    pub type_signature: TypeSignature,
}

#[derive(Clone)]
pub struct NativeFunctionOutput {
    pub documentation: String,
    pub type_signature: TypeSignature,
}

#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub documentation: String,
    pub inputs: Vec<NativeFunctionInput>,
    pub output: NativeFunctionOutput,
    pub example: String,
    pub snippet: String,
    pub run: NativeFunctionRun,
    pub check: NativeFunctionCheck,
}

type NativeFunctionRun = fn(&NativeFunction, Vec<Value>) -> Value;
type NativeFunctionCheck = fn(&NativeFunction, Vec<TypeSignature>) -> TypeSignature;

pub trait FunctionImplementation {
    fn check(ctx: &NativeFunction, args: Vec<TypeSignature>) -> TypeSignature;
    fn run(ctx: &NativeFunction, args: Vec<Value>) -> Value;
}
