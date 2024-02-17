use serde::{Serialize, Serializer};
#[derive(Clone, Debug)]
pub enum Value {
    String(String),
    UnsignedInteger(u64),
    SignedInteger(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::String(val) => serializer.serialize_str(val),
            Value::UnsignedInteger(val) => serializer.serialize_u64(*val),
            Value::SignedInteger(val) => serializer.serialize_i64(*val),
            Value::Float(val) => serializer.serialize_f64(*val),
            Value::Bool(val) => serializer.serialize_bool(*val),
            Value::Null => serializer.serialize_none(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub enum Typing {
    String,
    UnsignedInteger,
    SignedInteger,
    Float,
    Bool,
    Null,
}
