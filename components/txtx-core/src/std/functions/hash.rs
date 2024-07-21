use kit::sha2::Sha256 as LibSha256;
use kit::types::types::{TypeImplementation, TypeSpecification};
use ripemd::{Digest, Ripemd160 as LibRipemd160};

use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

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
        }
    ];
    pub static ref HASH_BUFFER: TypeSpecification = define_addon_type! {
        HashBuffer => {
            name: "hash_buffer",
            documentation: "Hash Buffer",
        }
    };
}

pub struct Ripemd160;
impl FunctionImplementation for Ripemd160 {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let Some(value) = args.get(0) else {
            return Err(diagnosed_error!("{}: expected 1 argument, got 0", ctx.name));
        };

        let mut hasher = LibRipemd160::new();
        hasher.update(value.to_bytes());
        let result = hasher.finalize();
        let value = Value::buffer(result[..].to_vec(), HASH_BUFFER.clone());
        Ok(value)
    }
}

pub struct Sha256;
impl FunctionImplementation for Sha256 {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let Some(value) = args.get(0) else {
            return Err(diagnosed_error!("{}: expected 1 argument, got 0", ctx.name));
        };

        let mut hasher = LibSha256::new();
        hasher.update(value.to_bytes());
        let result = hasher.finalize();
        let value = Value::buffer(result[..].to_vec(), HASH_BUFFER.clone());
        Ok(value)
    }
}

pub struct HashBuffer;
impl TypeImplementation for HashBuffer {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
