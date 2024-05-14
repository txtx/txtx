use clarity::vm::types::{
    ASCIIData, BuffData, CharType, PrincipalData, SequenceData, SequencedValue, UTF8Data,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{PrimitiveValue, Type, Value},
};

use crate::{
    stacks_helpers::value_to_tuple,
    typing::{
        CLARITY_ASCII, CLARITY_BUFFER, CLARITY_INT, CLARITY_PRINCIPAL, CLARITY_TUPLE, CLARITY_UINT,
        CLARITY_UTF8,
    },
};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeClarityValueOk => {
                name: "encode_ok",
                documentation: "`encode_ok` returns the given Clarity value wrapped in an `Ok` Clarity type.",
                example: "encode_ok(encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "The input wrapped in an `Ok` Clarity type.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            EncodeClarityValueErr => {
                name: "encode_err",
                documentation: "`encode_err` returns the given Clarity value wrapped in an `Err` Clarity type.",
                example: "encode_err(encode_uint(1))",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "The input wrapped in an `Err` Clarity type.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            EncodeClarityValueUint => {
                name: "encode_uint",
                documentation: "`encode_uint` returns the given number as a Clarity `uint`.",
                example: "encode_uint(1)",
                inputs: [
                    clarity_value: {
                        documentation: "A positive integer between 0 and 2<sup>128</sup>-1.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The input integer as a Clarity `uint`.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            EncodeClarityValueInt => {
                name: "encode_int",
                documentation: "`encode_int` returns the given number as a Clarity `int`.",
                example: "encode_int(1)",
                inputs: [
                    clarity_value: {
                        documentation: "An integer between -2<sup>127</sup> and 2<sup>127</sup>-1.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The input integer as a Clarity `int`.",
                    typing: Type::int()
                },
            }
        },
        define_function! {
            EncodeClarityValuePrincipal => {
                name: "encode_principal",
                documentation: "`encode_principal` returns the given string as a Clarity `principal`. A Clarity `principal` represents a Stacks address on the blockchain. Clarity admits two different kinds of principals: _standard principals_ and _contract principals_. Standard principals are backed by a corresponding private key whilst contract principals point to a smart contract.",
                example: "encode_principal(\"SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE\")",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity principal string.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity `principal`.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            EncodeClarityValueAscii => {
                name: "encode_string_ascii",
                documentation: "`encode_string_ascii` returns the given string as a Clarity ASCII string.",
                example: "encode_string_ascii(\"my ascii string\")",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid ASCII string.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity ASCII string.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            EncodeClarityValueUTF8 => {
                name: "encode_string_utf8",
                documentation: "`encode_string_utf8` returns the given string as a Clarity UTF-8 string.",
                example: "encode_string_utf8(\"🍊\")",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid UTF-8 string.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity UTF-8 string.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            EncodeClarityValueTuple => {
                name: "encode_tuple",
                documentation: "`encode_tuple` returns the given object as a Clarity tuple.",
                example: "encode_tuple({ \"key\": encode_uint(1) })",
                inputs: [
                    clarity_value: {
                        documentation: "An object where each key is a string and each value is a valid Clarity value.",
                        typing: vec![Type::object(vec![])]
                    }
                ],
                output: {
                    documentation: "The input object as a Clarity tuple.",
                    typing: Type::int()
                },
            }
        },
        define_function! {
            EncodeClarityValueBuffer => {
                name: "encode_buffer",
                documentation: "`encode_buffer` returns the given hex string as a Clarity buffer.",
                example: "encode_buffer(\"0x010203\")",
                inputs: [
                    clarity_value: {
                        documentation: "A hex string.",
                        typing: vec![Type::object(vec![])]
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity buffer.",
                    typing: Type::int()
                },
            }
        },
    ];
}

pub struct EncodeClarityValueOk;
impl FunctionImplementation for EncodeClarityValueOk {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        Ok(Value::bool(true))
    }
}

pub struct EncodeClarityValueErr;
impl FunctionImplementation for EncodeClarityValueErr {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        Ok(Value::bool(true))
    }
}

#[derive(Clone)]
pub struct StacksEncodeSome;
impl FunctionImplementation for StacksEncodeSome {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeNone;
impl FunctionImplementation for StacksEncodeNone {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeBool;
impl FunctionImplementation for StacksEncodeBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct EncodeClarityValueUint;
impl FunctionImplementation for EncodeClarityValueUint {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
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
pub struct EncodeClarityValueInt;
impl FunctionImplementation for EncodeClarityValueInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value = ClarityValue::Int(i128::from(*entry));
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_INT.clone()))
    }
}

pub struct EncodeClarityValuePrincipal;
impl FunctionImplementation for EncodeClarityValuePrincipal {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value = ClarityValue::Principal(PrincipalData::parse(&entry).unwrap());
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_PRINCIPAL.clone()))
    }
}

pub struct EncodeClarityValueAscii;
impl FunctionImplementation for EncodeClarityValueAscii {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value =
            ClarityValue::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
                data: entry.as_bytes().to_vec(),
            })));
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_ASCII.clone()))
    }
}

pub struct EncodeClarityValueUTF8;
impl FunctionImplementation for EncodeClarityValueUTF8 {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value = UTF8Data::to_value(&entry.as_bytes().to_vec());
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UTF8.clone()))
    }
}

pub struct EncodeClarityValueBuffer;
impl FunctionImplementation for EncodeClarityValueBuffer {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let data = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => {
                if val.starts_with("0x") {
                    txtx_addon_kit::hex::decode(&val[2..]).unwrap()
                } else {
                    txtx_addon_kit::hex::decode(&val[0..]).unwrap()
                }
            }
            _ => unreachable!(),
        };

        let bytes =
            ClarityValue::Sequence(SequenceData::Buffer(BuffData { data })).serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_BUFFER.clone()))
    }
}

pub struct EncodeClarityValueTuple;
impl FunctionImplementation for EncodeClarityValueTuple {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let clarity_value = match args.get(0) {
            Some(value) => ClarityValue::Tuple(value_to_tuple(value)),
            _ => unreachable!(),
        };
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_TUPLE.clone()))
    }
}

pub struct StacksEncodeInt;
impl FunctionImplementation for StacksEncodeInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeBuffer;
impl FunctionImplementation for StacksEncodeBuffer {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeList;
impl FunctionImplementation for StacksEncodeList {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeAsciiString;
impl FunctionImplementation for StacksEncodeAsciiString {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodePrincipal;
impl FunctionImplementation for StacksEncodePrincipal {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct StacksEncodeTuple;
impl FunctionImplementation for StacksEncodeTuple {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}
