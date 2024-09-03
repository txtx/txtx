use std::collections::BTreeMap;

use clarity::vm::{
    types::{
        signatures::CallableSubtype, BuffData, CallableData, CharType, ListData, ResponseData,
        SequenceData, SequencedValue, TupleData, TupleTypeSignature, TypeSignature as ClarityType,
    },
    ClarityName,
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::typing::StacksValue;

pub fn decode_cv_bytes(bytes: &Vec<u8>) -> Result<ClarityValue, String> {
    match ClarityValue::consensus_deserialize(&mut &bytes[..]) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("failed to parse clarity value: {}", e.to_string())),
    }
}

fn cv_to_clarity_type(cv: &ClarityValue) -> Result<ClarityType, String> {
    let ct = match cv {
        ClarityValue::Int(_) => ClarityType::IntType,
        ClarityValue::UInt(_) => ClarityType::UIntType,
        ClarityValue::Bool(_) => ClarityType::BoolType,
        ClarityValue::Sequence(s) => sequence_type_signature(s)?,
        ClarityValue::Principal(_) => ClarityType::PrincipalType,
        ClarityValue::Tuple(t) => ClarityType::TupleType(t.type_signature.clone()),
        ClarityValue::Optional(t) => t
            .type_signature()
            .map_err(|e| format!("failed to extract clarity optional type: {e}"))?,
        ClarityValue::Response(t) => t
            .type_signature()
            .map_err(|e| format!("failed to extract clarity response type: {e}"))?,
        ClarityValue::CallableContract(c) => {
            ClarityType::CallableType(callable_data_to_sub_type(c))
        }
    };
    Ok(ct)
}

fn sequence_type_signature(seq: &SequenceData) -> Result<ClarityType, String> {
    match seq {
        SequenceData::Buffer(b) => {
            b.type_signature().map_err(|e| format!("failed to extract clarity buffer type: {e}"))
        }
        SequenceData::List(l) => {
            l.type_signature().map_err(|e| format!("failed to extract clarity list type: {e}"))
        }
        SequenceData::String(s) => match s {
            CharType::UTF8(b) => {
                b.type_signature().map_err(|e| format!("failed to extract clarity utf8 type: {e}"))
            }
            CharType::ASCII(a) => {
                a.type_signature().map_err(|e| format!("failed to extract clarity ascii type: {e}"))
            }
        },
    }
}

fn callable_data_to_sub_type(callable: &CallableData) -> CallableSubtype {
    match &callable.trait_identifier.clone() {
        Some(trait_id) => CallableSubtype::Trait(trait_id.clone()),
        None => CallableSubtype::Principal(callable.contract_identifier.clone()),
    }
}

pub fn value_to_tuple(value: &Value) -> Result<TupleData, String> {
    match value {
        Value::Object(props) => {
            let mut type_map = BTreeMap::new();
            let mut data_map = BTreeMap::new();
            for (k, value) in props.into_iter() {
                let clarity_name = ClarityName::try_from(k.clone())
                    .map_err(|e| format!("invalid clarity tuple key {}: {}", k, e))?;
                let cv = value_to_cv(&value)?;
                let ct = cv_to_clarity_type(&cv)?;
                type_map.insert(clarity_name.clone(), ct);
                data_map.insert(clarity_name.clone(), cv);
            }

            Ok(TupleData {
                type_signature: TupleTypeSignature::try_from(type_map)
                    .map_err(|e| format!("invalid clarity tuple: {}", e))?,
                data_map,
            })
        }
        _ => {
            Err(format!("clarity tuple must be an object, found {}", value.get_type().to_string()))
        }
    }
}

pub fn cv_to_value(clarity_value: ClarityValue) -> Result<Value, Diagnostic> {
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
            let values =
                data.into_iter().map(|v| cv_to_value(v)).collect::<Result<Vec<_>, Diagnostic>>()?;
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
                let cv = cv_to_value(v)?;
                map.insert(k.to_string(), cv);
            }
            Ok(Value::Object(map))
        }
        ClarityValue::Optional(value) => match value.data {
            Some(value) => cv_to_value(*value),
            None => Ok(Value::null()),
        },
        ClarityValue::Response(ResponseData { data, .. }) => cv_to_value(*data),
        ClarityValue::CallableContract(val) => Ok(Value::String(val.to_string())),
    }
}

pub fn value_to_cv(src: &Value) -> Result<ClarityValue, String> {
    let dst = match src {
        Value::Addon(addon_data) => decode_cv_bytes(&addon_data.bytes)?,
        Value::Array(array) => {
            // should be encoded to list
            let mut values = vec![];
            for element in array.iter() {
                let value = value_to_cv(element)?;
                values.push(value);
            }
            ClarityValue::cons_list_unsanitized(values)
                .map_err(|e| format!("unable to encode Clarity list: {}", e.to_string()))?
        }
        Value::String(_) => {
            if let Some(bytes) = src.try_get_buffer_bytes_result()? {
                ClarityValue::buff_from(bytes)
                    .map_err(|e| format!("unable to encode Clarity buffer: {}", e.to_string()))?
            } else {
                return Err(format!("unable to infer typing (ascii vs utf8). Use stacks::cv_string_utf8(<value>) or stacks::cv_string_ascii(<value>) to reduce ambiguity."));
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
                return Err(format!("unable to infer typing (signed vs unsigned). Use stacks::cv_uint(<value>) or stacks::cv_int(<value>) to reduce ambiguity."));
            }
        }
        Value::Buffer(data) => ClarityValue::buff_from(data.clone())
            .map_err(|e| format!("unable to encode Clarity buffer: {}", e.to_string()))?,
        Value::Float(_) => {
            return Err(format!("unable to encode float to a Clarity type"));
        }
        Value::Object(object) => {
            let mut data = vec![];
            for (key, value) in object.iter() {
                let tuple_value = value_to_cv(&value.clone())?;
                let tuple_key = ClarityName::try_from(key.as_str()).map_err(|e| {
                    format!("unable to encode key {} to clarity type: {}", key, e.to_string())
                })?;
                data.push((tuple_key, tuple_value));
            }
            let tuple_data = TupleData::from_data(data)
                .map_err(|e| format!("unable to encode tuple data: {}", e.to_string()))?;
            ClarityValue::Tuple(tuple_data)
        }
    };
    Ok(dst)
}

pub fn txid_display_str(txid: &str) -> String {
    format!("{first_six}...{last_six}", first_six = &txid[0..6], last_six = &txid[txid.len() - 6..],)
}
