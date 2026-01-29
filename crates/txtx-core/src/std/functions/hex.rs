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

#[cfg(test)]
mod tests {
    use test_case::test_case;
    use txtx_addon_kit::helpers::fs::FileLocation;

    use super::*;

    fn get_spec_by_name(name: &str) -> FunctionSpecification {
        FUNCTIONS.iter().find(|f| f.name == name).cloned().unwrap()
    }

    fn dummy_auth_ctx() -> AuthorizationContext {
        AuthorizationContext { workspace_location: FileLocation::working_dir() }
    }

    #[test_case(
        Value::buffer(b"hello, world".to_vec()),
        Value::string("0x68656c6c6f2c20776f726c64".to_string());
        "buffer hello world"
    )]
    #[test_case(
        Value::string("0x68656c6c6f2c20776f726c64".to_string()),
        Value::string("0x68656c6c6f2c20776f726c64".to_string());
        "hex string passthrough"
    )]
    #[test_case(
        Value::buffer(vec![]),
        Value::string("0x".to_string());
        "empty buffer"
    )]
    #[test_case(
        Value::buffer(vec![255]),
        Value::string("0xff".to_string());
        "single byte max value"
    )]
    #[test_case(
        Value::buffer(vec![0]),
        Value::string("0x00".to_string());
        "single byte zero"
    )]
    #[test_case(
        Value::buffer(vec![0, 1, 127, 128, 254, 255]),
        Value::string("0x00017f80feff".to_string());
        "binary data with edge bytes"
    )]
    fn test_hex_encode_decode_roundtrip(input: Value, expected_encoded: Value) {
        let encode_spec = get_spec_by_name("encode_hex");
        let decode_spec = get_spec_by_name("decode_hex");
        let auth_ctx = dummy_auth_ctx();

        // Encode the input and verify it matches expected
        let encoded = (encode_spec.runner)(&encode_spec, &auth_ctx, &vec![input.clone()]).unwrap();
        assert_eq!(encoded, expected_encoded, "encoded value mismatch");

        // Decode the result and verify we get back the original bytes
        let decoded = (decode_spec.runner)(&decode_spec, &auth_ctx, &vec![encoded]).unwrap();
        let expected_buffer = Value::buffer(input.get_buffer_bytes_result().unwrap());
        assert_eq!(decoded, expected_buffer, "decoded value mismatch");
    }

    #[test_case(Value::string("0xGGGG".to_string()); "invalid hex chars")]
    #[test_case(Value::string("0x123".to_string()); "odd length hex")]
    fn test_decode_hex_invalid_input(input: Value) {
        let fn_spec = get_spec_by_name("decode_hex");
        let result = (fn_spec.runner)(&fn_spec, &dummy_auth_ctx(), &vec![input]);
        assert!(result.is_err());
    }
}
