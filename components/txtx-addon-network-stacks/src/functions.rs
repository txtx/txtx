use txtx_addon_kit::types::functions::{
    FunctionDeclaration, FunctionImplementation, Typing, Value,
};

lazy_static! {
    pub static ref STACKS_FUNCTIONS: Vec<FunctionDeclaration> = vec![
        define_native_function! {
            StacksEncodeOk => {
                name: "stacks_encode_ok",
                documentation: "Encode data",
                example: "stacks_encode_ok(stacks_encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Ok Clarity type",
                    typing: Typing::Bool
                },
            }
        },
        define_native_function! {
            StacksEncodeErr => {
                name: "stacks_encode_err",
                documentation: "Encode data",
                example: "stacks_encode_ok(stacks_encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Err Clarity type",
                    typing: Typing::Bool
                },
            }
        },
    ];
}

pub struct StacksEncodeOk;
impl FunctionImplementation for StacksEncodeOk {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        println!("Executing {}", ctx.name);
        Value::Bool(true)
    }
}

pub struct StacksEncodeErr;
impl FunctionImplementation for StacksEncodeErr {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        println!("Executing {}", ctx.name);
        Value::Bool(true)
    }
}

#[derive(Clone)]
pub struct StacksEncodeSome;
impl FunctionImplementation for StacksEncodeSome {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeNone;
impl FunctionImplementation for StacksEncodeNone {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeBool;
impl FunctionImplementation for StacksEncodeBool {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeUint;
impl FunctionImplementation for StacksEncodeUint {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeInt;
impl FunctionImplementation for StacksEncodeInt {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeBuffer;
impl FunctionImplementation for StacksEncodeBuffer {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeList;
impl FunctionImplementation for StacksEncodeList {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeAsciiString;
impl FunctionImplementation for StacksEncodeAsciiString {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodePrincipal;
impl FunctionImplementation for StacksEncodePrincipal {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeTuple;
impl FunctionImplementation for StacksEncodeTuple {
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        unimplemented!()
    }
}
