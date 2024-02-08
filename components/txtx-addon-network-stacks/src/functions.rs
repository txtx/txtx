use txtx_addon_kit::types::{
    functions::{FunctionImplementation, FunctionSpecification},
    typing::{Typing, Value},
};

lazy_static! {
    pub static ref STACKS_FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeClarityValueOk => {
                name: "clarity_value_ok",
                documentation: "Encode data",
                example: "stacks_encode_ok(stacks_encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Typing::Bool]
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Ok Clarity type",
                    typing: Typing::Bool
                },
            }
        },
        define_function! {
            EncodeClarityValueErr => {
                name: "clarity_value_err",
                documentation: "Wra",
                example: "stacks_encode_err(stacks_encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Typing::Bool]
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

pub struct EncodeClarityValueOk;
impl FunctionImplementation for EncodeClarityValueOk {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        Value::Bool(true)
    }
}

pub struct EncodeClarityValueErr;
impl FunctionImplementation for EncodeClarityValueErr {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        Value::Bool(true)
    }
}

#[derive(Clone)]
pub struct StacksEncodeSome;
impl FunctionImplementation for StacksEncodeSome {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeNone;
impl FunctionImplementation for StacksEncodeNone {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeBool;
impl FunctionImplementation for StacksEncodeBool {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeUint;
impl FunctionImplementation for StacksEncodeUint {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeInt;
impl FunctionImplementation for StacksEncodeInt {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeBuffer;
impl FunctionImplementation for StacksEncodeBuffer {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeList;
impl FunctionImplementation for StacksEncodeList {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeAsciiString;
impl FunctionImplementation for StacksEncodeAsciiString {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodePrincipal;
impl FunctionImplementation for StacksEncodePrincipal {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct StacksEncodeTuple;
impl FunctionImplementation for StacksEncodeTuple {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}
