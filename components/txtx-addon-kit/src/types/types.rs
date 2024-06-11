use jaq_interpret::Val;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use super::diagnostics::Diagnostic;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    Primitive(PrimitiveValue),
    Object(HashMap<String, Result<Value, Diagnostic>>),
    Array(Box<Vec<Value>>),
    Addon(Box<AddonData>),
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
    pub fn array(array: Vec<Value>) -> Value {
        Value::Array(Box::new(array))
    }
    pub fn object(object: HashMap<String, Result<Value, Diagnostic>>) -> Value {
        Value::Object(object)
    }

    pub fn expect_string(&self) -> &str {
        match &self {
            Value::Primitive(PrimitiveValue::String(value)) => value,
            _ => unreachable!(),
        }
    }
    pub fn expect_uint(&self) -> u64 {
        match &self {
            Value::Primitive(PrimitiveValue::UnsignedInteger(value)) => *value,
            _ => unreachable!(),
        }
    }
    pub fn expect_int(&self) -> i64 {
        match &self {
            Value::Primitive(PrimitiveValue::SignedInteger(value)) => *value,
            _ => unreachable!(),
        }
    }
    pub fn expect_float(&self) -> f64 {
        match &self {
            Value::Primitive(PrimitiveValue::Float(value)) => *value,
            _ => unreachable!(),
        }
    }
    pub fn expect_null(&self) -> () {
        match &self {
            Value::Primitive(PrimitiveValue::Null) => (),
            _ => unreachable!(),
        }
    }
    pub fn expect_bool(&self) -> bool {
        match &self {
            Value::Primitive(PrimitiveValue::Bool(value)) => *value,
            _ => unreachable!(),
        }
    }
    pub fn expect_buffer_data(&self) -> &BufferData {
        match &self {
            Value::Primitive(PrimitiveValue::Buffer(value)) => &value,
            _ => unreachable!(),
        }
    }
    pub fn expect_buffer_bytes(&self) -> Vec<u8> {
        match &self {
            Value::Primitive(PrimitiveValue::Buffer(value)) => value.bytes.clone(),
            Value::Primitive(PrimitiveValue::String(bytes)) => {
                let bytes = if bytes.starts_with("0x") {
                    crate::hex::decode(&bytes[2..]).unwrap()
                } else {
                    crate::hex::decode(&bytes).unwrap()
                };
                bytes
            }
            Value::Array(values) => values
                .iter()
                .flat_map(|v| v.expect_buffer_bytes())
                .collect(),
            _ => unreachable!(),
        }
    }
    pub fn expect_array(&self) -> &Box<Vec<Value>> {
        match &self {
            Value::Array(value) => value,
            _ => unreachable!(),
        }
    }

    pub fn expect_object(&self) -> &HashMap<String, Result<Value, Diagnostic>> {
        match &self {
            Value::Object(value) => value,
            _ => unreachable!(),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match &self {
            Value::Primitive(PrimitiveValue::String(value)) => Some(value),
            _ => None,
        }
    }
    pub fn as_uint(&self) -> Option<u64> {
        match &self {
            Value::Primitive(PrimitiveValue::UnsignedInteger(value)) => Some(*value),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match &self {
            Value::Primitive(PrimitiveValue::SignedInteger(value)) => Some(*value),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f64> {
        match &self {
            Value::Primitive(PrimitiveValue::Float(value)) => Some(*value),
            _ => None,
        }
    }
    pub fn as_null(&self) -> Option<()> {
        match &self {
            Value::Primitive(PrimitiveValue::Null) => Some(()),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match &self {
            Value::Primitive(PrimitiveValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }
    pub fn as_buffer_data(&self) -> Option<&BufferData> {
        match &self {
            Value::Primitive(PrimitiveValue::Buffer(value)) => Some(&value),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Box<Vec<Value>>> {
        match &self {
            Value::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Result<Value, Diagnostic>>> {
        match &self {
            Value::Object(value) => Some(value),
            _ => None,
        }
    }
    pub fn expect_primitive(&self) -> &PrimitiveValue {
        match &self {
            Value::Primitive(primitive) => primitive,
            _ => unreachable!(),
        }
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
            (Value::Array(_), Value::Primitive(_)) => false,
            (Value::Array(_), Value::Object(_)) => false,
            (Value::Array(lhs), Value::Array(rhs)) => {
                let Some(first_lhs) = lhs.first() else {
                    return false;
                };
                let Some(first_rhs) = rhs.first() else {
                    return false;
                };
                first_lhs.is_type_eq(first_rhs)
            }
            (Value::Addon(_), Value::Primitive(_)) => false,
            (Value::Addon(_), Value::Object(_)) => false,
            (Value::Addon(lhs), Value::Addon(rhs)) => lhs.typing.id == rhs.typing.id,
            (Value::Array(_), Value::Addon(_)) => false,
            (Value::Addon(_), Value::Array(_)) => false,
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self {
            Value::Primitive(PrimitiveValue::Buffer(buf)) => buf.bytes.clone(),
            Value::Array(values) => {
                let mut joined = vec![];
                for value in values.iter() {
                    joined.extend(value.to_bytes());
                }
                joined
            }
            Value::Primitive(PrimitiveValue::String(bytes)) => {
                let bytes = if bytes.starts_with("0x") {
                    crate::hex::decode(&bytes[2..]).unwrap()
                } else {
                    crate::hex::decode(&bytes).unwrap()
                };
                bytes
            }
            _ => unimplemented!(),
        }
    }
}

impl Value {
    pub fn from_string(
        value: String,
        expected_type: Type,
        typing: Option<TypeSpecification>,
    ) -> Result<Value, Diagnostic> {
        match expected_type {
            Type::Primitive(primitive_type) => {
                match PrimitiveValue::from_string(value, primitive_type, typing) {
                    Ok(v) => Ok(Value::Primitive(v)),
                    Err(e) => Err(e),
                }
            }
            Type::Object(_) => todo!(),
            Type::Addon(_) => todo!(),
            Type::Array(_) => todo!(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Primitive(PrimitiveValue::String(val)) => val.clone(),
            Value::Primitive(PrimitiveValue::Bool(val)) => val.to_string(),
            Value::Primitive(PrimitiveValue::SignedInteger(val)) => val.to_string(),
            Value::Primitive(PrimitiveValue::UnsignedInteger(val)) => val.to_string(),
            Value::Primitive(PrimitiveValue::Float(val)) => val.to_string(),
            Value::Primitive(PrimitiveValue::Null) => "null".to_string(),
            Value::Primitive(PrimitiveValue::Buffer(val)) => {
                format!("0x{}", hex::encode(&val.bytes))
            }
            Value::Object(_) => todo!(),
            Value::Array(_) => todo!(),
            Value::Addon(_) => todo!(),
        }
    }

    pub fn from_jaq_value(value: Val) -> Value {
        match value {
            Val::Null => Value::null(),
            Val::Bool(val) => Value::bool(val),
            Val::Int(val) => Value::int(i64::try_from(val).unwrap()),
            Val::Float(val) => Value::float(val),
            Val::Num(val) => Value::uint(val.parse().unwrap()),
            Val::Str(val) => Value::string(val.to_string()),
            Val::Arr(val) => {
                let mut arr = vec![];
                val.iter().for_each(|v| {
                    arr.push(Value::from_jaq_value(v.clone()));
                });
                Value::array(arr)
            }
            Val::Obj(val) => {
                let mut obj = HashMap::new();
                val.iter().for_each(|(k, v)| {
                    obj.insert(k.to_string(), Ok(Value::from_jaq_value(v.clone())));
                });
                Value::Object(obj)
            }
        }
    }
    pub fn get_type(&self) -> Type {
        match self {
            Value::Primitive(t) => Type::Primitive(t.get_type()),
            Value::Object(_) => todo!(),
            Value::Array(t) => {
                Type::Array(Box::new(t.first().unwrap_or(&Value::null()).get_type()))
            }
            Value::Addon(t) => Type::Addon(t.typing.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum PrimitiveValue {
    String(String),
    #[serde(rename = "UInt")]
    UnsignedInteger(u64),
    #[serde(rename = "Int")]
    SignedInteger(i64),
    Float(f64),
    Bool(bool),
    Null,
    Buffer(BufferData),
}

// impl Serialize for PrimitiveValue {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match self {
//             PrimitiveValue::String(val) => serializer.serialize_str(val),
//             PrimitiveValue::UnsignedInteger(val) => serializer.serialize_u64(*val),
//             PrimitiveValue::SignedInteger(val) => serializer.serialize_i64(*val),
//             PrimitiveValue::Float(val) => serializer.serialize_f64(*val),
//             PrimitiveValue::Bool(val) => serializer.serialize_bool(*val),
//             PrimitiveValue::Null => serializer.serialize_none(),
//             PrimitiveValue::Buffer(BufferData { bytes, typing: _ }) => {
//                 let mut s = String::from("0x");
//                 s.write_str(
//                     &bytes
//                         .iter()
//                         .map(|b| format!("{:02X}", b))
//                         .collect::<String>(),
//                 )
//                 .unwrap();
//                 serializer.serialize_str(&s)
//             }
//         }
//     }
// }

impl PrimitiveValue {
    pub fn from_string(
        value: String,
        expected_type: PrimitiveType,
        typing: Option<TypeSpecification>,
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
            PrimitiveType::Buffer => match hex::decode(&value) {
                Ok(bytes) => Ok(PrimitiveValue::Buffer(BufferData {
                    bytes,
                    typing: typing.unwrap(),
                })),
                Err(e) => unimplemented!("failed to cast {} to buffer: {}", value, e),
            },
        }
    }

    pub fn get_type(&self) -> PrimitiveType {
        match self {
            PrimitiveValue::String(_) => PrimitiveType::String,
            PrimitiveValue::UnsignedInteger(_) => PrimitiveType::UnsignedInteger,
            PrimitiveValue::SignedInteger(_) => PrimitiveType::SignedInteger,
            PrimitiveValue::Float(_) => PrimitiveType::Float,
            PrimitiveValue::Bool(_) => PrimitiveType::Bool,
            PrimitiveValue::Null => PrimitiveType::Null,
            PrimitiveValue::Buffer(_) => PrimitiveType::Buffer,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BufferData {
    pub bytes: Vec<u8>,
    pub typing: TypeSpecification,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AddonData {
    pub value: Value,
    pub typing: TypeSpecification,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Object(Vec<ObjectProperty>),
    Addon(TypeSpecification),
    Array(Box<Type>),
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
    pub fn array(array_item_type: Type) -> Type {
        Type::Array(Box::new(array_item_type))
    }
}

impl From<String> for Type {
    fn from(value: String) -> Self {
        match value.as_str() {
            "String" => Type::string(),
            "UInt" => Type::uint(),
            "Int" => Type::int(),
            "Float" => Type::float(),
            "Boolean" => Type::bool(),
            "Null" => Type::null(),
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
            Type::Primitive(PrimitiveType::String) => serializer.serialize_str("String"),
            Type::Primitive(PrimitiveType::UnsignedInteger) => serializer.serialize_str("UInt"),
            Type::Primitive(PrimitiveType::SignedInteger) => serializer.serialize_str("Int"),
            Type::Primitive(PrimitiveType::Float) => serializer.serialize_str("Float"),
            Type::Primitive(PrimitiveType::Bool) => serializer.serialize_str("Boolean"),
            Type::Primitive(PrimitiveType::Null) => serializer.serialize_str("Null"),
            Type::Primitive(PrimitiveType::Buffer) => serializer.serialize_str("Buffer"),
            Type::Object(_) => serializer.serialize_str("Object"),
            Type::Addon(a) => serializer.serialize_newtype_variant("Type", 3, "Addon", a),
            Type::Array(v) => serializer.serialize_newtype_variant("Type", 4, "Array", v),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    #[serde(rename = "UInt")]
    UnsignedInteger,
    #[serde(rename = "Int")]
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
    pub typing: Type,
    pub optional: bool,
    pub interpolable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize)]
pub struct TypeSpecification {
    pub id: String,
    pub documentation: String,
    #[serde(skip, default = "default_checker")]
    pub checker: TypeChecker,
}

// todo: think through this for serde
fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
    todo!();
}
fn default_checker() -> TypeChecker {
    check
}

impl Serialize for TypeSpecification {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("documentation", &self.documentation)?;
        map.end()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Refinements {
    pub specs: BTreeMap<String, Type>,
}

type TypeChecker = fn(&TypeSpecification, lhs: &Type, rhs: &Type) -> Result<bool, Diagnostic>;
pub trait TypeImplementation {
    fn check(_ctx: &TypeSpecification, lhs: &Type, rhs: &Type) -> Result<bool, Diagnostic>;
}
