use std::collections::{BTreeMap, HashMap};

use clarity::vm::{
    types::{
        ASCIIData, BuffData, BufferLength, CharType, ListData, ResponseData, SequenceData,
        SequenceSubtype, StringSubtype, StringUTF8Length, TupleData, TupleTypeSignature,
        TypeSignature as ClarityType,
    },
    ClarityName,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{BufferData, PrimitiveValue, TypeSpecification, Value},
};

use crate::typing::CLARITY_BUFFER;

pub fn parse_clarity_value(
    bytes: &Vec<u8>,
    typing: &TypeSpecification,
) -> Result<ClarityValue, Diagnostic> {
    match typing.id.as_str() {
        "clarity_uint" | "clarity_int" | "clarity_bool" | "clarity_tuple" | "clarity_principal"
        | "clarity_ascii" | "clarity_utf8" | "clarity_buffer" | "clarity_ok" | "clarity_value" => {
            match ClarityValue::consensus_deserialize(&mut &bytes[..]) {
                Ok(v) => Ok(v),
                Err(e) => Err(Diagnostic::error_from_string(format!(
                    "failed to parse clarity value: {}",
                    e.to_string()
                ))),
            }
        }
        _ => {
            unimplemented!()
        }
    }
}

fn extract_clarity_type(typing: &TypeSpecification, value: &Value) -> ClarityType {
    match typing.id.as_str() {
        "clarity_uint" => ClarityType::UIntType,
        "clarity_int" => ClarityType::IntType,
        "clarity_principal" => ClarityType::PrincipalType,
        "clarity_ascii" => match value {
            Value::Primitive(PrimitiveValue::String(value)) => {
                ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    BufferLength::try_from(value.len()).unwrap(),
                )))
            }
            Value::Primitive(PrimitiveValue::Buffer(buffer_data)) => {
                ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    BufferLength::try_from(buffer_data.bytes.len()).unwrap(),
                )))
            }
            v => unreachable!("clarity ascii values cannot be derived from value {:?}", v),
        },
        "clarity_utf8" => match value {
            Value::Primitive(PrimitiveValue::String(value)) => {
                ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    StringUTF8Length::try_from(value.len()).unwrap(),
                )))
            }
            Value::Primitive(PrimitiveValue::Buffer(buffer_data)) => {
                ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    StringUTF8Length::try_from(buffer_data.bytes.len()).unwrap(),
                )))
            }
            v => unreachable!("clarity ascii values cannot be derived from value {:?}", v),
        },
        "clarity_tuple" => {
            let tuple_data = value_to_tuple(value);
            ClarityType::TupleType(tuple_data.type_signature)
        }
        "clarity_ok" => todo!("implement clarity_ok"),
        "clarity_buffer" => todo!("implement clarity_buffer"),
        "clarity_list" => todo!("implement clarity_list"),
        _ => ClarityType::NoType,
    }
}

// todo: return result,diag
pub fn value_to_tuple(value: &Value) -> TupleData {
    match value {
        Value::Object(props) => {
            let mut type_map = BTreeMap::new();
            let mut data_map = BTreeMap::new();
            for (k, v) in props.into_iter() {
                let clarity_name = ClarityName::try_from(k.clone()).unwrap();
                let (clarity_type, clarity_value) = match v {
                    Ok(Value::Addon(addon)) => {
                        let clarity_type = extract_clarity_type(&addon.typing, &addon.value);
                        let clarity_value = clarity_type_and_primitive_to_clarity_value(
                            &clarity_type,
                            &addon.value,
                        );
                        (clarity_type, clarity_value)
                    }
                    Ok(Value::Primitive(PrimitiveValue::Buffer(buffer))) => {
                        let clarity_value =
                            parse_clarity_value(&buffer.bytes, &buffer.typing).unwrap();
                        let clarity_type = extract_clarity_type(
                            &buffer.typing,
                            &Value::Primitive(PrimitiveValue::Buffer(buffer.clone())),
                        );
                        (clarity_type, clarity_value)
                    }
                    Ok(Value::Primitive(PrimitiveValue::Bool(bool))) => {
                        let clarity_type = ClarityType::BoolType;
                        let clarity_value = ClarityValue::Bool(*bool);
                        (clarity_type, clarity_value)
                    }
                    Ok(v) => unimplemented!("{:?}", v),
                    Err(e) => unimplemented!("{}", e),
                };
                type_map.insert(clarity_name.clone(), clarity_type);
                data_map.insert(clarity_name.clone(), clarity_value);
            }

            TupleData {
                type_signature: TupleTypeSignature::try_from(type_map).unwrap(),
                data_map: data_map,
            }
        }
        v => unimplemented!(
            "tuple extraction is only supported for object types, got {:?}",
            v
        ),
    }
}

fn clarity_type_and_primitive_to_clarity_value(
    typing: &ClarityType,
    value: &Value,
) -> ClarityValue {
    match (typing, value) {
        (ClarityType::UIntType, Value::Primitive(PrimitiveValue::UnsignedInteger(v))) => {
            ClarityValue::UInt(u128::from(*v))
        }
        (ClarityType::IntType, Value::Primitive(PrimitiveValue::SignedInteger(v))) => {
            ClarityValue::Int(i128::from(*v))
        }
        (ClarityType::BoolType, Value::Primitive(PrimitiveValue::Bool(v))) => {
            ClarityValue::Bool(*v)
        }
        (
            ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))),
            Value::Primitive(PrimitiveValue::String(v)),
        ) => ClarityValue::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
            data: v.as_bytes().to_vec(),
        }))),
        (t, v) => unimplemented!("value {:?} cannot be casted to clarity type {}", v, t),
    }
}

pub fn clarity_value_to_value(clarity_value: ClarityValue) -> Result<Value, Diagnostic> {
    match clarity_value {
        ClarityValue::Int(val) => match i64::try_from(val) {
            Ok(val) => Ok(Value::Primitive(PrimitiveValue::SignedInteger(val))),
            Err(e) => Err(Diagnostic::error_from_string(format!(
                "failed to convert clarity value {}: {}",
                val, e
            ))),
        },
        ClarityValue::UInt(val) => match u64::try_from(val) {
            Ok(val) => Ok(Value::Primitive(PrimitiveValue::UnsignedInteger(val))),
            Err(e) => Err(Diagnostic::error_from_string(format!(
                "failed to convert clarity value {}: {}",
                val, e
            ))),
        },
        ClarityValue::Bool(val) => Ok(Value::Primitive(PrimitiveValue::Bool(val))),
        ClarityValue::Sequence(SequenceData::List(ListData { data, .. })) => {
            let values = data
                .into_iter()
                .map(|v| clarity_value_to_value(v))
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            Ok(Value::Array(Box::new(values)))
        }
        ClarityValue::Sequence(SequenceData::Buffer(BuffData { data })) => {
            Ok(Value::Primitive(PrimitiveValue::Buffer(BufferData {
                bytes: data,
                typing: CLARITY_BUFFER.clone(),
            })))
        }
        ClarityValue::Sequence(SequenceData::String(val)) => {
            let string = val.to_string();
            Ok(Value::Primitive(PrimitiveValue::String(string)))
        }
        ClarityValue::Principal(val) => {
            Ok(Value::Primitive(PrimitiveValue::String(val.to_string())))
        }
        ClarityValue::Tuple(TupleData { data_map, .. }) => {
            let mut map = HashMap::new();
            data_map.into_iter().for_each(|(k, v)| {
                map.insert(k.to_string(), clarity_value_to_value(v));
            });
            Ok(Value::Object(map))
        }
        ClarityValue::Optional(_) => todo!(),
        ClarityValue::Response(ResponseData { data, .. }) => clarity_value_to_value(*data),
        ClarityValue::CallableContract(val) => {
            Ok(Value::Primitive(PrimitiveValue::String(val.to_string())))
        }
    }
}
