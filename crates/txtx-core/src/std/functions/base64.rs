use base64::{engine::general_purpose, Engine};
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
        Base64Decode => {
            name: "base64_decode",
            documentation: "`base64_decode` decodes a base64 encoded string and returns the result as a hex string.",
            example: indoc!{r#"
            output "decoded" { 
                value = base64_decode("SGVsbG8gd29ybGQh")
            }
            > decoded: 0x48656c6c6f20776f726c6421
          "#},
            inputs: [
                base64_string: {
                    documentation: "The base64 string to decode.",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "The decoded base64 string, as a hex string.",
                typing: Type::string()
            },
        }
    },];
}

pub struct Base64Decode;
impl FunctionImplementation for Base64Decode {
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
        let encoded = args.get(0).unwrap().expect_string();
        let decoded = general_purpose::STANDARD.decode(encoded).map_err(|e| {
            Diagnostic::error_from_string(format!(
                "failed to decode base64 string {}: {}",
                encoded, e
            ))
        })?;
        let decoded = txtx_addon_kit::hex::encode(decoded);
        Ok(Value::string(format!("0x{decoded}")))
    }
}
