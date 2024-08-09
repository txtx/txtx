use txtx_addon_kit::types::types::{Type, Value};

pub const STD_HASH: &str = "std::hash";

pub struct StdValue {}

impl StdValue {
    pub fn hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STD_HASH)
    }
}
