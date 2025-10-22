use ripemd::{Digest, Ripemd160 as LibRipemd160};
use txtx_addon_kit::keccak_hash::keccak;
use txtx_addon_kit::sha2::Sha256 as LibSha256;
use txtx_addon_kit::types::AuthorizationContext;

use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

use super::arg_checker;
use crate::std::typing::StdValue;

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            Ripemd160 => {
                name: "ripemd160",
                documentation: "`ripemd160` computes the Ripemd160 hash of a value.",
                example: indoc!{r#"
                output "hashed_data" {
                    value = ripemd160(encode_hex("hello, world"))
                }
                // > hashed_data: 0XA3201F82FCA034E46D10CD7B27E174976E241DA2
              "#},
                inputs: [
                    value: {
                        documentation: "The hex-encoded value to hash.",
                        typing: vec![Type::buffer(), Type::array(Type::buffer())]
                    }
                ],
                output: {
                    documentation: "The hashed result.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            Sha256 => {
                name: "sha256",
                documentation: "`sha256` computes the sha256 hash of a value.",
                example: indoc!{r#"
                output "hashed_data" {
                    value = sha256(encode_hex("hello, world"))
                }
                // > hashed_data: 0x09ca7e4eaa6e8ae9c7d261167129184883644d07dfba7cbfbc4c8a2e08360d5b
              "#},
                inputs: [
                    value: {
                        documentation: "The hex-encoded value to hash.",
                        typing: vec![Type::buffer(), Type::array(Type::buffer())]
                    }
                ],
                output: {
                    documentation: "The hashed result.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            Keccak256 => {
                name: "keccak256",
                documentation: "`std::keccak256` computes the keccak256 hash of a value.",
                example: indoc!{r#"
                output "hashed_data" {
                    value = keccak256("hello, world")
                }
                // > hashed_data: 0x09ca7e4eaa6e8ae9c7d261167129184883644d07dfba7cbfbc4c8a2e08360d5b
              "#},
                inputs: [
                    value: {
                        documentation: "The string value to hash.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The hashed result.",
                    typing: Type::string()
                },
            }
        }
    ];
}

pub struct Ripemd160;
impl FunctionImplementation for Ripemd160 {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        let Some(value) = args.get(0) else {
            return Err(diagnosed_error!("{}: expected 1 argument, got 0", fn_spec.name));
        };

        let mut hasher = LibRipemd160::new();
        hasher.update(value.to_be_bytes());
        let result = hasher.finalize();
        Ok(StdValue::hash(result[..].to_vec()))
    }
}

pub struct Sha256;
impl FunctionImplementation for Sha256 {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        let Some(value) = args.get(0) else {
            return Err(diagnosed_error!("{}: expected 1 argument, got 0", fn_spec.name));
        };

        let mut hasher = LibSha256::new();
        hasher.update(value.to_be_bytes());
        let result = hasher.finalize();
        Ok(StdValue::hash(result[..].to_vec()))
    }
}

pub struct Keccak256;
impl FunctionImplementation for Keccak256 {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let value = args.get(0).unwrap().as_string().unwrap().to_string();
        let hash = keccak(value.as_bytes());
        Ok(StdValue::hash(hash.0.to_vec()))
    }
}
