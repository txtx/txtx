use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{Type, Value},
    AuthorizationContext,
};

use crate::{
    codec::BitcoinOpcode,
    typing::{BitcoinValue, BITCOIN_OPCODE},
};
lazy_static! {
    pub static ref BITWISE_LOGIC_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        EqualVerify => {
            name: "op_equalverify",
            documentation: "`btc::op_equalverify` pushes the `OP_EQUALVERIFY` opcode onto the stack.",
            example: indoc! {r#"
                output "opcode" {
                    value = btc::op_equalverify()
                }                
                // > opcode: 0x88
            "#},
            inputs: [],
            output: {
                documentation: "A hex representation of the `OP_EQUALVERIFY` opcode.",
                typing: Type::addon(BITCOIN_OPCODE)
            },
        }
    },];
}

#[derive(Clone)]
pub struct EqualVerify;
impl FunctionImplementation for EqualVerify {
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
        Ok(BitcoinValue::opcode(BitcoinOpcode::OpEqualVerify.get_code()))
    }
}
