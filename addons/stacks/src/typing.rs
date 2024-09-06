use txtx_addon_kit::types::types::{Type, Value};

pub const STACKS_CV_UINT: &str = "stacks::cv_uint";
pub const STACKS_CV_INT: &str = "stacks::cv_int";
pub const STACKS_CV_BOOL: &str = "stacks::cv_bool";
pub const STACKS_CV_PRINCIPAL: &str = "stacks::cv_principal";
pub const STACKS_CV_STRING_ASCII: &str = "stacks::cv_string_ascii";
pub const STACKS_CV_STRING_UTF8: &str = "stacks::cv_string_utf8";
pub const STACKS_CV_TUPLE: &str = "stacks::cv_tuple";
pub const STACKS_CV_OK: &str = "stacks::cv_ok";
pub const STACKS_CV_ERR: &str = "stacks::cv_err";
pub const STACKS_CV_NONE: &str = "stacks::cv_none";
pub const STACKS_CV_SOME: &str = "stacks::cv_some";
pub const STACKS_CV_LIST: &str = "stacks::cv_list";
pub const STACKS_CV_BUFFER: &str = "stacks::cv_buffer";
pub const STACKS_CV_GENERIC: &str = "stacks::cv_generic";
pub const STACKS_POST_CONDITIONS: &str = "stacks::post_conditions";
pub const STACKS_POST_CONDITION_MODE: &str = "stacks::post_condition_mode";
pub const STACKS_TRANSACTION: &str = "stacks::transaction";
pub const STACKS_TRANSACTION_PAYLOAD: &str = "stacks::transaction_payload";
pub const STACKS_SIGNATURE: &str = "stacks::signature";

pub struct StacksValue {}

impl StacksValue {
    pub fn bool(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_BOOL)
    }

    pub fn int(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_INT)
    }

    pub fn uint(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_UINT)
    }

    pub fn buffer(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_BUFFER)
    }

    pub fn tuple(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_TUPLE)
    }

    pub fn string_ascii(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_STRING_ASCII)
    }

    pub fn string_utf8(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_STRING_UTF8)
    }

    pub fn some(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_SOME)
    }

    pub fn none(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_NONE)
    }

    pub fn ok(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_OK)
    }

    pub fn err(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_ERR)
    }

    pub fn principal(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_PRINCIPAL)
    }

    pub fn post_conditions(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_POST_CONDITIONS)
    }

    pub fn generic_clarity_value(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_CV_GENERIC)
    }

    pub fn transaction_payload(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_TRANSACTION_PAYLOAD)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_TRANSACTION)
    }

    pub fn signature(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, STACKS_SIGNATURE)
    }
}

lazy_static! {
    pub static ref DEPLOYMENT_ARTIFACTS_TYPE: Type = define_object_type! {
        contract_source: {
            documentation: "The contract source.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        contract_name: {
            documentation: "The contract name.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        clarity_version: {
            documentation: "The contract version.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        }
    };
}
