use kit::types::types::{TypeImplementation, TypeSpecification};
use ripemd::{Digest, Ripemd160 as LibRipemd160};
use sha2::Sha256 as LibSha256;

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
                documentation: "Coming soon",
                example: indoc!{r#"
              "#},
                inputs: [
                    value: {
                        documentation: "Coming soon",
                        typing: vec![Type::buffer(), Type::array(Type::buffer())]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            Sha256 => {
                name: "sha256",
                documentation: "Coming soon",
                example: indoc!{r#"
              "#},
                inputs: [
                    value: {
                        documentation: "Coming soon",
                        typing: vec![Type::buffer(), Type::array(Type::buffer())]
                    }
                ],
                output: {
                    documentation: "",
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
