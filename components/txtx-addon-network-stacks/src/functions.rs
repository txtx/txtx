use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{PrimitiveValue, Typing, Value},
};

use crate::typing::CLARITY_UINT;

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
                        typing: vec![Typing::bool()]
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Ok Clarity type",
                    typing: Typing::bool()
                },
            }
        },
        define_function! {
            EncodeClarityValueErr => {
                name: "clarity_value_err",
                documentation: "",
                example: "stacks_encode_err(stacks_encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Typing::bool()]
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Err Clarity type",
                    typing: Typing::bool()
                },
            }
        },
        define_function! {
            EncodeClarityValueUint => {
                name: "clarity_value_uint",
                documentation: "",
                example: "clarity_value_uint(1)",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::int()
                },
            }
        },
    ];
}

pub struct EncodeClarityValueOk;
impl FunctionImplementation for EncodeClarityValueOk {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        Ok(Value::bool(true))
    }
}

pub struct EncodeClarityValueErr;
impl FunctionImplementation for EncodeClarityValueErr {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        Ok(Value::bool(true))
    }
}

#[derive(Clone)]
pub struct StacksEncodeSome;
impl FunctionImplementation for StacksEncodeSome {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeNone;
impl FunctionImplementation for StacksEncodeNone {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeBool;
impl FunctionImplementation for StacksEncodeBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct EncodeClarityValueUint;
impl FunctionImplementation for EncodeClarityValueUint {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value = ClarityValue::UInt(u128::from(*entry));
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UINT.clone()))
    }
}

pub struct StacksEncodeInt;
impl FunctionImplementation for StacksEncodeInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeBuffer;
impl FunctionImplementation for StacksEncodeBuffer {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeList;
impl FunctionImplementation for StacksEncodeList {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeAsciiString;
impl FunctionImplementation for StacksEncodeAsciiString {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodePrincipal;
impl FunctionImplementation for StacksEncodePrincipal {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeTuple;
impl FunctionImplementation for StacksEncodeTuple {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}
