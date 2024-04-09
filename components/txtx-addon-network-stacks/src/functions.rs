use std::collections::BTreeMap;

use clarity::vm::{
    types::{
        ASCIIData, BufferLength, CharType, PrincipalData, SequenceData, SequenceSubtype,
        StringSubtype, TupleData, TupleTypeSignature, TypeSignature as ClarityType,
    },
    ClarityName,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{PrimitiveValue, Type, TypeSpecification, Value},
};

use crate::typing::{CLARITY_INT, CLARITY_PRINCIPAL, CLARITY_TUPLE, CLARITY_UINT};

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
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Ok Clarity type",
                    typing: Type::bool()
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
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "Input wrapped into an Err Clarity type",
                    typing: Type::bool()
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
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            EncodeClarityValueInt => {
                name: "clarity_value_int",
                documentation: "",
                example: "clarity_value_int(1)",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::int()
                },
            }
        },
        define_function! {
          EncodeClarityValuePrincipal => {
                name: "clarity_value_standard_principal",
                documentation: "",
                example: "clarity_value_standard_principal('SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE')",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::string()
                },
            }
        },
        define_function! {
          EncodeClarityValuePrincipal => {
                name: "clarity_value_contract_principal",
                documentation: "",
                example: "clarity_value_contract_principal('SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.arkadiko-stake-pool-diko-v1-2')",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::string()
                },
            }
        },
        define_function! {
          EncodeClarityValueAscii => {
                name: "clarity_value_ascii",
                documentation: "",
                example: "clarity_value_ascii('my ascii string')",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            EncodeClarityValueTuple => {
                name: "clarity_value_tuple",
                documentation: "",
                example: "clarity_value_tuple({ 'some_key': clarity_value_uint(1) })",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: vec![Type::object(vec![])]
                    }
                ],
                output: {
                    documentation: "",
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
        Ok(Value::buffer(bytes, CLARITY_PRINCIPAL.clone()))
    }
}

pub struct EncodeClarityValueTuple;
impl FunctionImplementation for EncodeClarityValueTuple {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let clarity_value = match args.get(0) {
            Some(Value::Object(props)) => {
                let mut type_map = BTreeMap::new();
                let mut data_map = BTreeMap::new();
                for (k, v) in props.into_iter() {
                    let clarity_name = ClarityName::try_from(k.clone()).unwrap();
                    match v {
                        Ok(PrimitiveValue::UnsignedInteger(v)) => {
                            let cv = ClarityValue::UInt(u128::from(*v));
                            type_map.insert(clarity_name.clone(), ClarityType::UIntType);
                            data_map.insert(clarity_name.clone(), cv);
                        }
                        Ok(PrimitiveValue::SignedInteger(v)) => {
                            let cv = ClarityValue::Int(i128::from(*v));
                            type_map.insert(clarity_name.clone(), ClarityType::IntType);
                            data_map.insert(clarity_name.clone(), cv);
                        }
                        Ok(PrimitiveValue::Bool(v)) => {
                            let cv = ClarityValue::Bool(*v);
                            type_map.insert(clarity_name.clone(), ClarityType::BoolType);
                            data_map.insert(clarity_name.clone(), cv);
                        }
                        Ok(PrimitiveValue::String(v)) => {
                            let cv = ClarityValue::Sequence(SequenceData::String(CharType::ASCII(
                                ASCIIData {
                                    data: v.as_bytes().to_vec(),
                                },
                            )));
                            type_map.insert(
                                clarity_name.clone(),
                                ClarityType::SequenceType(SequenceSubtype::StringType(
                                    StringSubtype::ASCII(
                                        BufferLength::try_from(cv.size()).unwrap(),
                                    ),
                                )),
                            );
                            data_map.insert(clarity_name.clone(), cv);
                        }
                        Ok(v) => unimplemented!("{:?}", v),
                        Err(e) => unimplemented!("{}", e),
                    }
                }

                ClarityValue::Tuple(TupleData {
                    type_signature: TupleTypeSignature::try_from(type_map).unwrap(),
                    data_map: data_map,
                })
            }
            _ => unreachable!(),
        };
        let bytes = clarity_value.serialize_to_vec();
        Ok(Value::buffer(bytes, CLARITY_TUPLE.clone()))
    }
}

pub fn parse_clarity_value(
    bytes: &Vec<u8>,
    typing: &TypeSpecification,
) -> Result<ClarityValue, Diagnostic> {
    match typing.id.as_str() {
        "clarity_uint" | "clarity_int" | "clarity_bool" | "clarity_tuple" | "clarity_principal" => {
            match ClarityValue::consensus_deserialize(&mut &bytes[..]) {
                Ok(v) => Ok(v),
                Err(e) => Err(Diagnostic::error_from_string(e.to_string())),
            }
        }
        _ => {
            unimplemented!()
        }
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
