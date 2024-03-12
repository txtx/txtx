use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use serde::{Serialize, Serializer};

use super::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub enum Value {
    Primitive(PrimitiveValue),
    Object(HashMap<String, Result<PrimitiveValue, Diagnostic>>),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Primitive(PrimitiveValue::String(val)) => serializer.serialize_str(val),
            Value::Primitive(PrimitiveValue::UnsignedInteger(val)) => {
                serializer.serialize_u64(*val)
            }
            Value::Primitive(PrimitiveValue::SignedInteger(val)) => serializer.serialize_i64(*val),
            Value::Primitive(PrimitiveValue::Float(val)) => serializer.serialize_f64(*val),
            Value::Primitive(PrimitiveValue::Bool(val)) => serializer.serialize_bool(*val),
            Value::Primitive(PrimitiveValue::Null) => serializer.serialize_none(),
            Value::Primitive(PrimitiveValue::Buffer(_)) => {
                unimplemented!("Value::Primitive(PrimitiveValue::Buffer) variant")
            }
            Value::Object(_) => unimplemented!("Value::Object variant"),
        }
    }
}

impl Value {
    pub fn string(value: String) -> Value {
        Value::Primitive(PrimitiveValue::String(value.to_string()))
    }
    pub fn uint(value: u64) -> Value {
        Value::Primitive(PrimitiveValue::UnsignedInteger(value))
    }
    pub fn int(value: i64) -> Value {
        Value::Primitive(PrimitiveValue::SignedInteger(value))
    }
    pub fn float(value: f64) -> Value {
        Value::Primitive(PrimitiveValue::Float(value))
    }
    pub fn null() -> Value {
        Value::Primitive(PrimitiveValue::Null)
    }
    pub fn bool(value: bool) -> Value {
        Value::Primitive(PrimitiveValue::Bool(value))
    }
    pub fn buffer(bytes: Vec<u8>, typing: TypeSpecification) -> Value {
        Value::Primitive(PrimitiveValue::Buffer(BufferData { bytes, typing }))
    }

    pub fn is_type_eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::Primitive(PrimitiveValue::Null), Value::Primitive(PrimitiveValue::Null)) => {
                true
            }
            (
                Value::Primitive(PrimitiveValue::Bool(_)),
                Value::Primitive(PrimitiveValue::Bool(_)),
            ) => true,
            (
                Value::Primitive(PrimitiveValue::UnsignedInteger(_)),
                Value::Primitive(PrimitiveValue::UnsignedInteger(_)),
            ) => true,
            (
                Value::Primitive(PrimitiveValue::SignedInteger(_)),
                Value::Primitive(PrimitiveValue::SignedInteger(_)),
            ) => true,
            (
                Value::Primitive(PrimitiveValue::Float(_)),
                Value::Primitive(PrimitiveValue::Float(_)),
            ) => true,
            (
                Value::Primitive(PrimitiveValue::String(_)),
                Value::Primitive(PrimitiveValue::String(_)),
            ) => true,
            (
                Value::Primitive(PrimitiveValue::Buffer(_)),
                Value::Primitive(PrimitiveValue::Buffer(_)),
            ) => true,
            (Value::Object(_), Value::Object(_)) => true,
            (Value::Primitive(PrimitiveValue::Null), _) => false,
            (Value::Primitive(PrimitiveValue::Bool(_)), _) => false,
            (Value::Primitive(PrimitiveValue::UnsignedInteger(_)), _) => false,
            (Value::Primitive(PrimitiveValue::SignedInteger(_)), _) => false,
            (Value::Primitive(PrimitiveValue::Float(_)), _) => false,
            (Value::Primitive(PrimitiveValue::String(_)), _) => false,
            (Value::Primitive(PrimitiveValue::Buffer(_)), _) => false,
            (Value::Object(_), _) => false,
        }
    }
}

impl Value {
    pub fn from_string(value: String, expected_type: Type) -> Result<Value, Diagnostic> {
        match expected_type {
            Type::Primitive(primitive_type) => {
                match PrimitiveValue::from_string(value, primitive_type) {
                    Ok(v) => Ok(Value::Primitive(v)),
                    Err(e) => Err(e),
                }
            }
            Type::Object(_) => todo!(),
            Type::Addon(_) => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PrimitiveValue {
    String(String),
    UnsignedInteger(u64),
    SignedInteger(i64),
    Float(f64),
    Bool(bool),
    Null,
    Buffer(BufferData),
}

impl PrimitiveValue {
    pub fn from_string(
        value: String,
        expected_type: PrimitiveType,
    ) -> Result<PrimitiveValue, Diagnostic> {
        match expected_type {
            PrimitiveType::String => Ok(PrimitiveValue::String(value)),
            PrimitiveType::UnsignedInteger => match value.parse() {
                Ok(value) => Ok(PrimitiveValue::UnsignedInteger(value)),
                Err(e) => unimplemented!("failed to cast {} to uint: {}", value, e),
            },
            PrimitiveType::SignedInteger => match value.parse() {
                Ok(value) => Ok(PrimitiveValue::SignedInteger(value)),
                Err(e) => unimplemented!("failed to cast {} to int: {}", value, e),
            },
            PrimitiveType::Float => match value.parse() {
                Ok(value) => Ok(PrimitiveValue::Float(value)),
                Err(e) => unimplemented!("failed to cast {} to float: {}", value, e),
            },
            PrimitiveType::Null => {
                if value.is_empty() {
                    Ok(PrimitiveValue::Null)
                } else {
                    unimplemented!("failed to cast {} to null", value,);
                }
            }
            PrimitiveType::Bool => match value.parse() {
                Ok(value) => Ok(PrimitiveValue::Bool(value)),
                Err(e) => unimplemented!("failed to cast {} to bool: {}", value, e),
            },
            PrimitiveType::Buffer => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BufferData {
    pub bytes: Vec<u8>,
    pub typing: TypeSpecification,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Object(Vec<ObjectProperty>),
    Addon(TypeSpecification),
}
impl Type {
    pub fn string() -> Type {
        Type::Primitive(PrimitiveType::String)
    }
    pub fn uint() -> Type {
        Type::Primitive(PrimitiveType::UnsignedInteger)
    }
    pub fn int() -> Type {
        Type::Primitive(PrimitiveType::SignedInteger)
    }
    pub fn float() -> Type {
        Type::Primitive(PrimitiveType::Float)
    }
    pub fn null() -> Type {
        Type::Primitive(PrimitiveType::Null)
    }
    pub fn bool() -> Type {
        Type::Primitive(PrimitiveType::Bool)
    }
    pub fn object(props: Vec<ObjectProperty>) -> Type {
        Type::Object(props)
    }
    pub fn buffer() -> Type {
        Type::Primitive(PrimitiveType::Buffer)
    }
    pub fn addon(type_spec: TypeSpecification) -> Type {
        Type::Addon(type_spec)
    }
}

impl From<String> for Type {
    fn from(value: String) -> Self {
        match value.as_str() {
            "string" => Type::string(),
            "uint" => Type::uint(),
            "int" => Type::int(),
            "float" => Type::float(),
            "boolean" => Type::bool(),
            "null" => Type::null(),
            _ => unimplemented!("Type from str not implemented"),
        }
    }
}

impl From<Option<String>> for Type {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(value) => Type::from(value),
            None => Type::default(),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::string()
    }
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Type::Primitive(PrimitiveType::String) => serializer.serialize_str("string"),
            Type::Primitive(PrimitiveType::UnsignedInteger) => serializer.serialize_str("uint"),
            Type::Primitive(PrimitiveType::SignedInteger) => serializer.serialize_str("int"),
            Type::Primitive(PrimitiveType::Float) => serializer.serialize_str("float"),
            Type::Primitive(PrimitiveType::Bool) => serializer.serialize_str("boolean"),
            Type::Primitive(PrimitiveType::Null) => serializer.serialize_str("null"),
            Type::Primitive(PrimitiveType::Buffer) => serializer.serialize_str("buffer"),
            Type::Object(_) => unimplemented!("Type::Object variant"),
            Type::Addon(_) => unimplemented!("Type::Addon variant"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    String,
    UnsignedInteger,
    SignedInteger,
    Float,
    Bool,
    Null,
    Buffer,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ObjectProperty {
    pub name: String,
    pub documentation: String,
    pub typing: PrimitiveType,
    pub optional: bool,
    pub interpolable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TypeSpecification {
    pub id: String,
    pub documentation: String,
    pub checker: TypeChecker,
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Refinements {
    pub specs: BTreeMap<String, Type>,
}

type TypeChecker = fn(&TypeSpecification, lhs: &Type, rhs: &Type) -> Result<bool, Diagnostic>;
pub trait TypeImplementation {
    fn check(_ctx: &TypeSpecification, lhs: &Type, rhs: &Type) -> Result<bool, Diagnostic>;
}
