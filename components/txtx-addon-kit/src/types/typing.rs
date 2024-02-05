#[derive(Clone, Debug)]
pub enum Value {
    String(String),
    Number(i64),
    Bool(bool),
}

#[derive(Clone, Debug)]
pub enum Typing {
    String,
    Number,
    Bool,
}
