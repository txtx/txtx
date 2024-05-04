use base64::{engine::general_purpose, Engine};
use txtx_addon_kit::{
    define_function,
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
            documentation: "",
            example: "base64_decode('UE5...')",
            inputs: [
                base64_string: {
                    documentation: "The base64 string to decode",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "The base64 decoded string",
                typing: Type::string()
            },
        }
    },];
}

pub struct Base64Decode;
impl FunctionImplementation for Base64Decode {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let encoded = args.get(0).unwrap().expect_string();
        let decoded = general_purpose::STANDARD.decode(encoded).map_err(|e| {
            Diagnostic::error_from_string(format!(
                "failed to decode base64 string {}: {}",
                encoded, e
            ))
        })?;
        let decoded = hex::encode(decoded);
        Ok(Value::string(format!("0x{decoded}")))
    }
}
