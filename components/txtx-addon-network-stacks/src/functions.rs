use clarity::vm::types::{
    ASCIIData, BuffData, CharType, OptionalData, PrincipalData, SequenceData, SequencedValue,
    UTF8Data,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{PrimitiveValue, Type, Value},
};

use crate::{
    stacks_helpers::{parse_clarity_value, value_to_tuple},
    typing::{
        CLARITY_ASCII, CLARITY_BUFFER, CLARITY_INT, CLARITY_PRINCIPAL, CLARITY_TUPLE, CLARITY_UINT,
        CLARITY_UTF8, CLARITY_VALUE,
    },
};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeClarityValueSome => {
                name: "cv_some",
                documentation: "`stacks::cv_some` wraps the given Clarity value in a Clarity `Optional`.",
                example: indoc! {r#"
                output "some" { 
                  value = stacks::cv_some(stacks::cv_bool(true))
                }
                // > some: 0x0a03
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "A Clarity Value.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The input Clarity value wrapped in a Clarity `Optional`.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            EncodeClarityValueNone => {
                name: "cv_none",
                documentation: "`stacks::cv_none` returns the Clarity value `None`.",
                example: indoc! {r#"
                output "none" { 
                  value = stacks::cv_none()
                }
                // > none: 0x09
                "#},
                inputs: [],
                output: {
                    documentation: "The Clarity value `None`.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            EncodeClarityValueBool => {
                name: "cv_bool",
                documentation: "`stacks::cv_bool` returns the given boolean as a Clarity `bool`.",
                example: indoc! {r#"
                output "my_bool" { 
                  value = stacks::cv_bool(true)
                }
                // > my_bool: 0x03
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "The boolean values `true` or `false`.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The input boolean as a Clarity `bool`.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            EncodeClarityValueUint => {
                name: "cv_uint",
                documentation: "`stacks::cv_uint` returns the given number as a Clarity `uint`.",
                example: indoc! {r#"
                output "my_uint" { 
                  value = stacks::cv_uint(1)
                }
                // > my_uint: 0x0100000000000000000000000000000001
                "#},
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
                name: "cv_int",
                documentation: "`stacks::cv_int` returns the given number as a Clarity `int`.",
                example: indoc! {r#"
                output "my_int" { 
                  value = stacks::cv_int(-1)
                }
                // > my_int: 0x00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
                "#},
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
                name: "cv_principal",
                documentation: txtx_addon_kit::indoc! {r#"
                `stacks::cv_principal` returns the given string as a Clarity principal. 
                A Clarity principal represents a Stacks address on the blockchain.

                Clarity admits two different kinds of principals: _standard principals_ and _contract principals_. 
                Standard principals (e.g. `SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE`) are backed by a corresponding private key while contract principals (e.g. `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1`) point to a smart contract.
                "#},
                example: indoc! {r#"
                output "my_principal" { 
                  value = stacks::cv_principal("SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE")
                }
                // > my_principal: 0x0516DEBC095099629BADB11B9D5335E874D12F1F1D45
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity principal string.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity principal.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            EncodeClarityValueAscii => {
                name: "cv_string_ascii",
                documentation: "`stacks::cv_string_ascii` returns the given string as a Clarity ASCII string.",
                example: indoc! {r#"
                output "my_ascii" { 
                  value = stacks::cv_string_ascii("my ascii string")
                }
                // > my_ascii: 0x0D0000000F6D7920617363696920737472696E67
                "#},
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
                name: "cv_string_utf8",
                documentation: "`stacks::cv_string_utf8` returns the given string as a Clarity UTF-8 string.",
                example: indoc! {r#"
                output "my_utf8" { 
                  value = stacks::cv_string_utf8("🍊")
                }
                // > my_utf8: 0x0E00000004F09F8D8A
                "#},
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
                name: "cv_tuple",
                documentation: "`stacks::cv_tuple` returns the given object as a Clarity tuple.",
                example: indoc! {r#"
                output "my_tuple" { 
                  value = stacks::cv_tuple({ "key": stacks::cv_uint(1) })
                }
                // > my_tuple: 0x0C00000001036B65790100000000000000000000000000000001
                "#},
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
                name: "cv_buff",
                documentation: "`stacks::cv_buff` returns the given hex string as a Clarity buffer.",
                example: indoc! {r#"
                output "my_buffer" { 
                  value = stacks::cv_buff("0x010203")
                }
                // > my_buffer: 0x0200000003010203
                "#},
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
        define_function! {
            EncodeClarityValueOk => {
                name: "cv_ok",
                documentation: "Coming soon - `stacks::cv_ok` returns the given Clarity value wrapped in an `Ok` Clarity type.",
                example: indoc! {r#"// Coming soon "#},
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
                name: "cv_err",
                documentation: "Coming soon - `stacks::cv_err` returns the given Clarity value wrapped in an `Err` Clarity type.",
                example: indoc! {r#"// Coming soon "#},
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
            DecodeClarityValueOk => {
                name: "decode_ok",
                documentation: "`stacks::decode_ok` returns the inner value as a Clarity buffer.",
                example: indoc! {r#"// Coming soon "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value.",
                        typing: vec![Type::buffer()]
                    }
                ],
                output: {
                    documentation: "The inner value that was wrapped in an `(ok <inner>)` Clarity type.",
                    typing: Type::buffer()
                },
            }
        },
    ];
}

pub struct EncodeClarityValueOk;
impl FunctionImplementation for EncodeClarityValueOk {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        Ok(Value::bool(true))
    }
}

pub struct EncodeClarityValueErr;
impl FunctionImplementation for EncodeClarityValueErr {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct EncodeClarityValueSome;
impl FunctionImplementation for EncodeClarityValueSome {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::Buffer(val))) => val.clone(),
            Some(any) => {
                return Err(diagnosed_error!(
                    "'cv_some' function: expected cv, got {:?}",
                    any
                ))
            }
            None => {
                return Err(diagnosed_error!(
                    "'cv_some' function: expected cv, got none :("
                ))
            }
        };
        let cv = match parse_clarity_value(&entry.bytes, &entry.typing) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let clarity_value = ClarityValue::Optional(OptionalData {
            data: Some(Box::new(cv)),
        });
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UINT.clone()))
    }
}

pub struct EncodeClarityValueNone;
impl FunctionImplementation for EncodeClarityValueNone {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        if !args.is_empty() {
            return Err(diagnosed_error!(
                "`cv_none` function: expected no arguments"
            ));
        }
        let clarity_value = ClarityValue::Optional(OptionalData { data: None });
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UINT.clone()))
    }
}

pub struct EncodeClarityValueBool;
impl FunctionImplementation for EncodeClarityValueBool {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::Bool(val))) => val.clone(),
            Some(any) => {
                return Err(diagnosed_error!(
                    "'cv_bool' function: expected bool, got {:?}",
                    any
                ))
            }
            None => {
                return Err(diagnosed_error!(
                    "'cv_bool' function: expected bool, got none :("
                ))
            }
        };
        let clarity_value = ClarityValue::Bool(entry);
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UINT.clone()))
    }
}

pub struct EncodeClarityValueUint;
impl FunctionImplementation for EncodeClarityValueUint {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) => val.clone(),
            Some(Value::Primitive(PrimitiveValue::SignedInteger(val))) => {
                let as_u64 = u64::try_from(val.clone()).map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "Failed to stacks::cv_uint, could not parse SignedInteger: {e}"
                    ))
                })?;
                as_u64
            }
            Some(any) => {
                return Err(diagnosed_error!(
                    "'cv_uint' function: expected uint, got {:?}",
                    any
                ))
            }
            None => {
                return Err(diagnosed_error!(
                    "'cv_uint' function: expected uint, got none :("
                ))
            }
        };
        let clarity_value = ClarityValue::UInt(u128::from(entry));
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_UINT.clone()))
    }
}
pub struct EncodeClarityValueInt;
impl FunctionImplementation for EncodeClarityValueInt {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::SignedInteger(val))) => val,
            _ => unreachable!(),
        };
        let clarity_value = ClarityValue::Int(i128::from(*entry));
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_INT.clone()))
    }
}

pub struct EncodeClarityValuePrincipal;
impl FunctionImplementation for EncodeClarityValuePrincipal {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
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
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
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
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
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
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
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
            Some(Value::Primitive(PrimitiveValue::Buffer(val))) => val.bytes.clone(),
            _ => unreachable!(),
        };

        let bytes =
            ClarityValue::Sequence(SequenceData::Buffer(BuffData { data })).serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_BUFFER.clone()))
    }
}

pub struct EncodeClarityValueTuple;
impl FunctionImplementation for EncodeClarityValueTuple {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
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

pub struct DecodeClarityValueOk;
impl FunctionImplementation for DecodeClarityValueOk {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let value = match args.get(0) {
            // todo maybe we can assume some types?
            Some(Value::Primitive(PrimitiveValue::Buffer(buffer_data))) => {
                match parse_clarity_value(&buffer_data.bytes, &buffer_data.typing) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                }
            }
            Some(Value::Primitive(PrimitiveValue::String(buffer_hex))) => {
                if !buffer_hex.starts_with("0x") {
                    unreachable!()
                }
                let bytes = txtx_addon_kit::hex::decode(&buffer_hex[2..]).unwrap();
                match parse_clarity_value(&bytes, &CLARITY_VALUE) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                }
            }
            Some(_v) => {
                return Err(diagnosed_error!(
                    "function '{}': argument type error",
                    &ctx.name
                ))
            }
            None => {
                return Err(diagnosed_error!(
                    "function '{}': argument missing",
                    &ctx.name
                ))
            }
        };

        let inner_bytes: Vec<u8> = value.serialize_to_vec();

        Ok(Value::buffer(inner_bytes, CLARITY_VALUE.clone()))
    }
}
