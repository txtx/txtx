use std::{collections::BTreeMap, fmt::Debug};
#[derive(Clone, Debug)]
pub enum Value {
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
    Native(NativeType),
    Addon(TypingDeclaration),
}
impl Typing {
    pub fn string() -> Typing {
        Typing::Native(NativeType::String)
    }
    pub fn uint() -> Typing {
        Typing::Native(NativeType::UnsignedInteger)
    }
    pub fn int() -> Typing {
        Typing::Native(NativeType::SignedInteger)
    }
    pub fn float() -> Typing {
        Typing::Native(NativeType::Float)
    }
    pub fn null() -> Typing {
        Typing::Native(NativeType::Null)
    }
    pub fn bool() -> Typing {
        Typing::Native(NativeType::Bool)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum NativeType {
    String,
    UnsignedInteger,
    SignedInteger,
    Float,
    Bool,
    Null,
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
