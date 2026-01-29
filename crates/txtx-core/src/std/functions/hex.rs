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
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeHex => {
                name: "encode_hex",
                documentation: "`encode_hex` encodes a buffer or string as a hexadecimal string with 0x prefix.",
                example: indoc!{r#"
                    output "encoded_hex" {
                        value = encode_hex("hello, world")
                    }
                    // > encoded_hex: 0x68656c6c6f2c20776f726c64
              "#},
                inputs: [
                    value: {
                        documentation: "The buffer or string to encode.",
                        typing: vec![Type::buffer(), Type::string(), Type::addon("any")]
                    }
                ],
                output: {
                    documentation: "The input in its hexadecimal representation with 0x prefix.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            DecodeHex => {
                name: "decode_hex",
                documentation: "`decode_hex` decodes a hexadecimal string and returns the result as a buffer.",
                example: indoc!{r#"
                    output "decoded_hex" {
                        value = decode_hex("0x68656c6c6f2c20776f726c64")
                    }
                    // > decoded_hex: 0x68656c6c6f2c20776f726c64
              "#},
                inputs: [
                    hex_string: {
                        documentation: "The hex string to decode.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The decoded hex string as a buffer.",
                    typing: Type::buffer()
                },
            }
        },
    ];
}

pub struct EncodeHex;
impl FunctionImplementation for EncodeHex {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let bytes = args
            .get(0)
            .unwrap()
            .get_buffer_bytes_result()
            .map_err(|e| Diagnostic::error_from_string(e))?;

        let hex = txtx_addon_kit::hex::encode(bytes);
        Ok(Value::string(format!("0x{}", hex)))
    }
}

pub struct DecodeHex;
impl FunctionImplementation for DecodeHex {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let hex_string = args.get(0).unwrap().expect_string();
        let hex_string = if hex_string.starts_with("0x") { &hex_string[2..] } else { hex_string };

        let bytes = txtx_addon_kit::hex::decode(hex_string).map_err(|e| {
            Diagnostic::error_from_string(format!(
                "failed to decode hex string {}: {}",
                hex_string, e
            ))
        })?;

        Ok(Value::buffer(bytes))
    }
}
