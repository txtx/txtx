#[derive(Clone, Debug)]
pub enum Value {
    String(String),
    UnsignedInteger(u64),
    SignedInteger(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Typing {
    String,
    UnsignedInteger,
    SignedInteger,
    Float,
    Bool,
    Null,
}
