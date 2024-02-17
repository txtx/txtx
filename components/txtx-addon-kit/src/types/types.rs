use std::{collections::BTreeMap, fmt::Debug};

use super::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub enum Value {
    Primitive(PrimitiveValue),
    Object(Vec<(String, Result<PrimitiveValue, Diagnostic>)>),
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

#[derive(Clone, Debug)]
pub struct BufferData {
    pub bytes: Vec<u8>,
    pub typing: Typing,
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Typing {
    Primitive(PrimitiveType),
    Object(Vec<ObjectProperty>),
    Addon(TypingDeclaration),
}
impl Typing {
    pub fn string() -> Typing {
        Typing::Primitive(PrimitiveType::String)
    }
    pub fn uint() -> Typing {
        Typing::Primitive(PrimitiveType::UnsignedInteger)
    }
    pub fn int() -> Typing {
        Typing::Primitive(PrimitiveType::SignedInteger)
    }
    pub fn float() -> Typing {
        Typing::Primitive(PrimitiveType::Float)
    }
    pub fn null() -> Typing {
        Typing::Primitive(PrimitiveType::Null)
    }
    pub fn bool() -> Typing {
        Typing::Primitive(PrimitiveType::Bool)
    }
    pub fn object(props: Vec<ObjectProperty>) -> Typing {
        Typing::Object(props)
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
pub struct TypingDeclaration {
    pub id: String,
    pub documentation: String,
    pub refinements: Refinements,
    pub check: TypeChecker,
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Refinements {
    pub specs: BTreeMap<String, Typing>,
}
type TypeChecker = fn(&TypingDeclaration, Vec<Typing>) -> (bool, Option<Typing>);
pub trait TypingImplementation {
    fn check(_ctx: &TypingDeclaration, lhs: &Typing, rhs: &Typing) -> bool;
}
