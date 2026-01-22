pub mod assertions;
pub mod base58;
pub mod base64;
pub mod big_endian;
pub mod crypto;
pub mod hash;
pub mod hex;
pub mod json;
pub mod list;
pub mod operators;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{arg_checker_with_ctx, fn_diag_with_ctx},
    types::Value,
};

use crate::constants::NAMESPACE;

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(NAMESPACE.to_string());
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(NAMESPACE.to_string());
    error_fn(fn_spec, e)
}

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
        functions.extend(base58::FUNCTIONS.clone());
        functions.extend(assertions::FUNCTIONS.clone());
        functions
    };
}
