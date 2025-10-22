use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{Type, Value},
    AuthorizationContext,
};

use crate::{codec::BitcoinOpcode, typing::BitcoinValue};

lazy_static! {
    pub static ref STACK_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        Dup => {
            name: "op_dup",
            documentation: "`btc::op_dup` pushes the `OP_DUP` opcode onto the stack.",
            example: indoc! {r#"
                output "opcode" {
                    value = btc::op_dup()
                }                
                // > opcode: 0x76
            "#},
            inputs: [],
            output: {
                documentation: "A hex representation of the `OP_DUP` opcode.",
                typing: Type::string()
            },
        }
    },];
}

#[derive(Clone)]
pub struct Dup;
impl FunctionImplementation for Dup {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Value],
    ) -> Result<Value, Diagnostic> {
        Ok(BitcoinValue::opcode(BitcoinOpcode::OpDup.get_code()))
    }
}
