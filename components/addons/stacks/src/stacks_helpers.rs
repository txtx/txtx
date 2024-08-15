use std::collections::BTreeMap;

use clarity::vm::{
    types::{
        ASCIIData, BuffData, BufferLength, CharType, ListData, ResponseData, SequenceData,
        SequenceSubtype, StringSubtype, StringUTF8Length, TupleData, TupleTypeSignature,
        TypeSignature as ClarityType,
    },
    ClarityName,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::typing::{
    StacksValue, STACKS_CV_BOOL, STACKS_CV_BUFFER, STACKS_CV_ERR, STACKS_CV_GENERIC, STACKS_CV_INT,
    STACKS_CV_LIST, STACKS_CV_NONE, STACKS_CV_OK, STACKS_CV_PRINCIPAL, STACKS_CV_SOME,
    STACKS_CV_STRING_ASCII, STACKS_CV_STRING_UTF8, STACKS_CV_TUPLE, STACKS_CV_UINT,
};

pub fn parse_clarity_value(bytes: &Vec<u8>, type_id: &str) -> Result<ClarityValue, Diagnostic> {
    match type_id {
        STACKS_CV_UINT
        | STACKS_CV_INT
        | STACKS_CV_BOOL
        | STACKS_CV_TUPLE
        | STACKS_CV_GENERIC
        | STACKS_CV_PRINCIPAL
        | STACKS_CV_STRING_ASCII
        | STACKS_CV_STRING_UTF8
        | STACKS_CV_BUFFER
        | STACKS_CV_OK
        | STACKS_CV_LIST
        | STACKS_CV_SOME
        | STACKS_CV_NONE
        | STACKS_CV_ERR => match ClarityValue::consensus_deserialize(&mut &bytes[..]) {
            Ok(v) => Ok(v),
            Err(e) => Err(Diagnostic::error_from_string(format!(
                "failed to parse clarity value: {}",
                e.to_string()
            ))),
        },
        _ => {
            unimplemented!()
        }
    }
}

fn extract_clarity_type(type_id: &str, value: &Value) -> ClarityType {
    match type_id {
        STACKS_CV_UINT => ClarityType::UIntType,
        STACKS_CV_INT => ClarityType::IntType,
        STACKS_CV_PRINCIPAL => ClarityType::PrincipalType,
        STACKS_CV_STRING_ASCII => match value {
            Value::String(value) => ClarityType::SequenceType(SequenceSubtype::StringType(
                StringSubtype::ASCII(BufferLength::try_from(value.len()).unwrap()),
            )),
            Value::Buffer(buffer_data) => ClarityType::SequenceType(SequenceSubtype::StringType(
                StringSubtype::ASCII(BufferLength::try_from(buffer_data.len()).unwrap()),
            )),
            v => unreachable!("clarity ascii values cannot be derived from value {:?}", v),
        },
        STACKS_CV_STRING_UTF8 => match value {
            Value::String(value) => ClarityType::SequenceType(SequenceSubtype::StringType(
                StringSubtype::UTF8(StringUTF8Length::try_from(value.len()).unwrap()),
            )),
            Value::Buffer(buffer_data) => ClarityType::SequenceType(SequenceSubtype::StringType(
                StringSubtype::UTF8(StringUTF8Length::try_from(buffer_data.len()).unwrap()),
            )),
            v => unreachable!("clarity ascii values cannot be derived from value {:?}", v),
        },
        STACKS_CV_TUPLE => {
            let tuple_data = value_to_tuple(value);
            ClarityType::TupleType(tuple_data.type_signature)
        }
        STACKS_CV_OK => todo!("implement clarity_ok"),
        STACKS_CV_BUFFER => todo!("implement clarity_buffer"),
        STACKS_CV_LIST => todo!("implement clarity_list"),
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
                    Value::Addon(addon) => {
                        let clarity_type = extract_clarity_type(&addon.id, &v);
                        let clarity_value =
                            clarity_type_and_primitive_to_clarity_value(&clarity_type, &v);
                        (clarity_type, clarity_value)
                    }
                    Value::Buffer(buffer) => {
                        let clarity_value =
                            parse_clarity_value(&buffer, &STACKS_CV_GENERIC).unwrap();
                        // let clarity_type = clarity_value
                        //     extract_clarity_type(&buffer.typing, &Value::buffer(buffer.clone()));
                        // (clarity_type, clarity_value)
                        todo!("implement clarity_buffer")
                    }
                    Value::Bool(bool) => {
                        let clarity_type = ClarityType::BoolType;
                        let clarity_value = ClarityValue::Bool(*bool);
                        (clarity_type, clarity_value)
                    }
                    v => unimplemented!("{:?}", v),
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
        (ClarityType::UIntType, Value::Integer(v)) => ClarityValue::UInt(u128::from(*v as u128)),
        (ClarityType::IntType, Value::Integer(v)) => ClarityValue::Int(i128::from(*v)),
        (ClarityType::BoolType, Value::Bool(v)) => ClarityValue::Bool(*v),
        (
            ClarityType::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))),
            Value::String(v),
        ) => ClarityValue::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
            data: v.as_bytes().to_vec(),
        }))),
        (t, v) => unimplemented!("value {:?} cannot be casted to clarity type {}", v, t),
    }
}

pub fn clarity_value_to_value(clarity_value: ClarityValue) -> Result<Value, Diagnostic> {
    match clarity_value {
        ClarityValue::Int(val) => match i64::try_from(val) {
            Ok(val) => Ok(Value::integer(val.into())),
            Err(e) => Err(Diagnostic::error_from_string(format!(
                "failed to convert clarity value {}: {}",
                val, e
            ))),
        },
        ClarityValue::UInt(val) => match u64::try_from(val) {
            Ok(val) => Ok(Value::integer(val.into())),
            Err(e) => Err(Diagnostic::error_from_string(format!(
                "failed to convert clarity value {}: {}",
                val, e
            ))),
        },
        ClarityValue::Bool(val) => Ok(Value::Bool(val)),
        ClarityValue::Sequence(SequenceData::List(ListData { data, .. })) => {
            let values = data
                .into_iter()
                .map(|v| clarity_value_to_value(v))
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            Ok(Value::Array(Box::new(values)))
        }
        ClarityValue::Sequence(SequenceData::Buffer(BuffData { data })) => {
            Ok(StacksValue::buffer(data))
        }
        ClarityValue::Sequence(SequenceData::String(val)) => {
            let string = val.to_string();
            Ok(Value::String(string))
        }
        ClarityValue::Principal(val) => Ok(Value::String(val.to_string())),
        ClarityValue::Tuple(TupleData { data_map, .. }) => {
            let mut map = IndexMap::new();
            for (k, v) in data_map.into_iter() {
                let cv = clarity_value_to_value(v)?;
                map.insert(k.to_string(), cv);
            }
            Ok(Value::Object(map))
        }
        ClarityValue::Optional(value) => match value.data {
            Some(value) => clarity_value_to_value(*value),
            None => Ok(Value::null()),
        },
        ClarityValue::Response(ResponseData { data, .. }) => clarity_value_to_value(*data),
        ClarityValue::CallableContract(val) => Ok(Value::String(val.to_string())),
    }
}

pub fn encode_any_value_to_clarity_value(src: &Value) -> Result<ClarityValue, Diagnostic> {
    let dst = match src {
        Value::Addon(addon_data) => {
            parse_clarity_value(&addon_data.bytes, &addon_data.id)?
        }
        Value::Array(array) => {
            // should be encoded to list
            let mut values = vec![];
            for element in array.iter() {
                let value = encode_any_value_to_clarity_value(element)?;
                values.push(value);
            }
            ClarityValue::list_from(values).map_err(|e| {
                diagnosed_error!("unable to encode Clarity list ({})", e.to_string())
            })?
        }
        Value::String(_) => {
            if let Some(bytes) = src.try_get_buffer_bytes() {
                ClarityValue::buff_from(bytes).map_err(|e| {
                    diagnosed_error!("unable to encode Clarity buffer ({})", e.to_string())
                })?
            } else {
                return Err(diagnosed_error!("unable to infer typing (ascii vs utf8). Use stacks::cv_string_utf8(<value>) or stacks::cv_string_ascii(<value>) to reduce ambiguity."));
            }
        }
        Value::Bool(value) => ClarityValue::Bool(*value),
        Value::Null => ClarityValue::none(),
        Value::Integer(int) => {
            if *int < 0 {
                ClarityValue::Int(*int)
            } else if *int > 0 {
                ClarityValue::UInt((*int).try_into().unwrap())
            } else {
                return Err(diagnosed_error!("unable to infer typing (signed vs unsigned). Use stacks::cv_uint(<value>) or stacks::cv_int(<value>) to reduce ambiguity."))
            }
        }
        Value::Buffer(data) => {
            // if data.typing.eq(&CLARITY_PRINCIPAL) {
            //     let value_bytes = data.bytes.clone();
            //     ClarityValue::consensus_deserialize(&mut &value_bytes[..])
            //         .map_err(|e| diagnosed_error!("{}", e.to_string()))?
            // } else {
                ClarityValue::buff_from(data.clone()).map_err(|e| {
                    diagnosed_error!("unable to encode Clarity buffer ({})", e.to_string())
                })?
            // }
        }
        Value::Float(_) => {
            // should return an error
            return Err(diagnosed_error!("unable to encode float to a Clarity type"));
        }
        Value::Object(object) => {
            // should be encoded as a tuple
            let mut data = vec![];
            for (key, value) in object.iter() {
                let tuple_value = encode_any_value_to_clarity_value(&value.clone())?;
                let tuple_key = ClarityName::try_from(key.as_str()).map_err(|e| {
                    diagnosed_error!(
                        "unable to encode key {} to clarity ({})",
                        key,
                        e.to_string()
                    )
                })?;
                data.push((tuple_key, tuple_value));
            }
            let tuple_data = TupleData::from_data(data)
                .map_err(|e| diagnosed_error!("unable to encode tuple data ({})", e.to_string()))?;
            ClarityValue::Tuple(tuple_data)
        }
    };
    Ok(dst)
}

pub fn txid_display_str(txid: &str) -> String {
    format!(
        "{first_six}...{last_six}",
        first_six = &txid[0..6],
        last_six = &txid[txid.len() - 6..],
    )
}
