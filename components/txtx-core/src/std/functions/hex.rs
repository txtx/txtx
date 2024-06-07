use kit::types::types::{TypeImplementation, TypeSpecification};
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
        EncodeHex => {
            name: "encode_hex",
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
    pub static ref STD_BUFFER: TypeSpecification = define_addon_type! {
        HashBuffer => {
            name: "std_buffer",
            documentation: "Standard Buffer",
        }
    };
}

pub struct EncodeHex;
impl FunctionImplementation for EncodeHex {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let input = args.get(0).unwrap().expect_string();
        let hex = kit::hex::encode(input);
        Ok(Value::string(hex))
    }
}

pub struct HashBuffer;
impl TypeImplementation for HashBuffer {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
