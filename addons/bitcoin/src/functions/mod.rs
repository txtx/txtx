use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{arg_checker_with_ctx, fn_diag_with_ctx, FunctionSpecification},
    namespace::Namespace,
    types::Value,
};

use crate::constants::NAMESPACE;

pub mod opcodes;

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &[Value]) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(Namespace::from(NAMESPACE));
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(Namespace::from(NAMESPACE));
    error_fn(fn_spec, e)
}

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = {
        let mut functions = vec![];
        functions.extend(opcodes::stack::STACK_FUNCTIONS.clone());
        functions.extend(opcodes::constants::CONSTANTS.clone());
        functions.extend(opcodes::control_flow::CONTROL_FLOW_FUNCTIONS.clone());
        functions.extend(opcodes::bitwise_logic::BITWISE_LOGIC_FUNCTIONS.clone());
        functions.extend(opcodes::crypto::CRYPTO_FUNCTIONS.clone());
        functions.extend(opcodes::arithmetic::ARITHMETIC_FUNCTIONS.clone());
        functions
    };
}
