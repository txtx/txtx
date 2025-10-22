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
    pub static ref CRYPTO_FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            Hash160 => {
                name: "op_hash160",
                documentation: "`btc::op_hash160` pushes the `OP_HASH160` opcode onto the stack.",
                example: indoc! {r#"
                    output "opcode" {
                        value = btc::op_hash160()
                    }                
                    // > opcode: 0xa9
                "#},
                inputs: [],
                output: {
                    documentation: "A hex representation of the `OP_HASH160` opcode.",
                    typing: Type::addon(BITCOIN_OPCODE)
                },
            }
        },
        define_function! {
            CheckSig => {
                name: "op_checksig",
                documentation: "`btc::op_checksig` pushes the `OP_CHECKSIG` opcode onto the stack.",
                example: indoc! {r#"
                    output "opcode" {
                        value = btc::op_hash160()
                    }                
                    // > opcode: 0xac
                "#},
                inputs: [],
                output: {
                    documentation: "A hex representation of the `OP_CHECKSIG` opcode.",
                    typing: Type::addon(BITCOIN_OPCODE)
                },
            }
        }
    ];
}

#[derive(Clone)]
pub struct Hash160;
impl FunctionImplementation for Hash160 {
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
        Ok(BitcoinValue::opcode(BitcoinOpcode::OpHash160.get_code()))
    }
}

#[derive(Clone)]
pub struct CheckSig;
impl FunctionImplementation for CheckSig {
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
        Ok(BitcoinValue::opcode(BitcoinOpcode::OpCheckSig.get_code()))
    }
}
