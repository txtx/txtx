pub mod post_conditions;
pub mod pre_conditions;

pub use post_conditions::*;
pub use pre_conditions::*;

use crate::types::types::Value;

use super::CommandSpecification;

const PRE_CONDITION: &str = "pre_condition";
const POST_CONDITION: &str = "post_condition";
const BEHAVIOR: &str = "behavior";
const ASSERTION: &str = "assertion";
const RETRIES: &str = "retries";
const BACKOFF: &str = "backoff";
const POST_CONDITION_ATTEMPTS: &str = "post_condition_attempts";
pub const ASSERTION_TYPE_ID: &str = "std::assertion";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionResult {
    Success,
    Failure(String),
}

impl AssertionResult {
    pub fn to_value(&self) -> Value {
        let bytes = serde_json::to_vec(self).unwrap();
        Value::addon(bytes, ASSERTION_TYPE_ID)
    }
    pub fn from_value(value: &Value) -> Result<Self, String> {
        match value {
            Value::Bool(bool) => {
                if *bool {
                    Ok(AssertionResult::Success)
                } else {
                    Err("assertion failed".to_string())
                }
            }
            Value::Addon(addon_data) => {
                if addon_data.id == ASSERTION_TYPE_ID {
                    serde_json::from_slice(&addon_data.bytes)
                        .map_err(|e| format!("failed to deserialize assertion: {}", e))
                } else {
                    Err(format!("expected type '{}', found '{}'", ASSERTION_TYPE_ID, addon_data.id))
                }
            }
            _ => Err(format!(
                "expected a boolean or an addon of type '{}', found '{}'",
                ASSERTION_TYPE_ID,
                value.get_type().to_string()
            )),
        }
    }
}
