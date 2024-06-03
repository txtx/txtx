use kit::types::types::{TypeImplementation, TypeSpecification};
use ripemd::{Digest, Ripemd160};
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        Base64Decode => {
            name: "ripemd160",
            documentation: "Coming soon",
            example: indoc!{r#"
          "#},
            inputs: [
                value: {
                    documentation: "Coming soon",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "",
                typing: Type::string()
            },
        }
    },];
    pub static ref HASH_BUFFER: TypeSpecification = define_addon_type! {
        HashBuffer => {
            name: "hash_buffer",
            documentation: "Hash Buffer",
        }
    };
}

pub struct Base64Decode;
impl FunctionImplementation for Base64Decode {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let input = args.get(0).unwrap().expect_string();
        let mut hasher = Ripemd160::new();
        hasher.update(input.as_bytes());
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
