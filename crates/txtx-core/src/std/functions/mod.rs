pub mod base64;
pub mod crypto;
pub mod hash;
pub mod hex;
pub mod json;
pub mod list;
pub mod operators;
use txtx_addon_kit::types::functions::FunctionSpecification;

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = {
        let mut functions = vec![];
        functions.extend(json::JSON_FUNCTIONS.clone());
        functions.extend(crypto::FUNCTIONS.clone());
        functions.extend(list::LIST_FUNCTIONS.clone());
        functions.extend(operators::OPERATORS_FUNCTIONS.clone());
        functions.extend(base64::FUNCTIONS.clone());
        functions.extend(hash::FUNCTIONS.clone());
        functions.extend(hex::FUNCTIONS.clone());
        functions
    };
}
