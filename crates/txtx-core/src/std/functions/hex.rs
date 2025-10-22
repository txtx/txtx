use txtx_addon_kit::types::AuthorizationContext;
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
            documentation: "`encode_hex` encodes a string as a hexadecimal string.",
            example: indoc!{r#"
                output "encoded_hex" {
                    value = encode_hex("hello, world")
                }
                // > encoded_hex: 68656C6C6F2C20776F726C64
          "#},
            inputs: [
                value: {
                    documentation: "Any input string.",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "The input string in its hexadecimal representation.",
                typing: Type::string()
            },
        }
    },];
}

pub struct EncodeHex;
impl FunctionImplementation for EncodeHex {
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
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        let input = args.get(0).unwrap().expect_string();
        let hex = txtx_addon_kit::hex::encode(input);
        Ok(Value::string(hex))
    }
}
