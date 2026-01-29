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
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            Base64Decode => {
                name: "decode_base64",
                documentation: "`decode_base64` decodes a base64 encoded string and returns the result as a buffer.",
                example: indoc!{r#"
                output "decoded" {
                    value = decode_base64("SGVsbG8gd29ybGQh")
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
                    documentation: "The decoded base64 string, as a buffer.",
                    typing: Type::buffer()
                },
            }
        },
        define_function! {
            Base64Encode => {
                name: "encode_base64",
                documentation: "`encode_base64` encodes a buffer or string as a base64 string.",
                example: indoc!{r#"
                output "encoded" {
                    value = encode_base64("0x48656c6c6f20776f726c6421")
                }
                > encoded: SGVsbG8gd29ybGQh
              "#},
                inputs: [
                    value: {
                        documentation: "The buffer or string to encode. Strings starting with '0x' are decoded as hex; otherwise, raw UTF-8 bytes are used.",
                        typing: vec![Type::buffer(), Type::string(), Type::addon("any")]
                    }
                ],
                output: {
                    documentation: "The input, encoded as a base64 string.",
                    typing: Type::string()
                },
            }
        },
    ];
}

pub struct Base64Decode;
impl FunctionImplementation for Base64Decode {
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
        let encoded = args.get(0).unwrap().expect_string();
        let decoded = general_purpose::STANDARD.decode(encoded).map_err(|e| {
            Diagnostic::error_from_string(format!(
                "failed to decode base64 string {}: {}",
                encoded, e
            ))
        })?;
        Ok(Value::buffer(decoded))
    }
}

/// Helper to get bytes from a Value for encoding functions.
/// - Buffer: use bytes directly
/// - String with "0x" prefix: decode as hex
/// - String without "0x" prefix: use raw UTF-8 bytes
/// - Addon: use addon bytes
fn get_bytes_for_encoding(value: &Value) -> Result<Vec<u8>, Diagnostic> {
    match value {
        Value::Buffer(b) => Ok(b.clone()),
        Value::String(s) => {
            if s.starts_with("0x") {
                txtx_addon_kit::hex::decode(&s[2..]).map_err(|e| {
                    Diagnostic::error_from_string(format!("failed to decode hex string: {}", e))
                })
            } else {
                Ok(s.as_bytes().to_vec())
            }
        }
        Value::Addon(addon) => Ok(addon.bytes.clone()),
        _ => Err(Diagnostic::error_from_string(
            "expected a buffer, string, or addon value".to_string(),
        )),
    }
}

pub struct Base64Encode;
impl FunctionImplementation for Base64Encode {
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
        let bytes = get_bytes_for_encoding(args.get(0).unwrap())?;
        let encoded = general_purpose::STANDARD.encode(bytes);
        Ok(Value::string(encoded))
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
        Value::buffer(b"Hello world!".to_vec()),
        Value::string("SGVsbG8gd29ybGQh".to_string());
        "buffer hello world"
    )]
    #[test_case(
        Value::string("Hello world!".to_string()),
        Value::string("SGVsbG8gd29ybGQh".to_string());
        "plain string hello world"
    )]
    #[test_case(
        Value::string("0x48656c6c6f20776f726c6421".to_string()),
        Value::string("SGVsbG8gd29ybGQh".to_string());
        "hex string hello world"
    )]
    #[test_case(
        Value::string("__event_authority".to_string()),
        Value::string("X19ldmVudF9hdXRob3JpdHk=".to_string());
        "plain string with underscores"
    )]
    #[test_case(
        Value::buffer(vec![]),
        Value::string("".to_string());
        "empty buffer"
    )]
    #[test_case(
        Value::buffer(vec![0, 1, 127, 128, 254, 255]),
        Value::string("AAF/gP7/".to_string());
        "binary data with edge bytes"
    )]
    fn test_base64_encode_decode_roundtrip(input: Value, expected_encoded: Value) {
        let encode_spec = get_spec_by_name("encode_base64");
        let decode_spec = get_spec_by_name("decode_base64");
        let auth_ctx = dummy_auth_ctx();

        // Encode the input and verify it matches expected
        let encoded = (encode_spec.runner)(&encode_spec, &auth_ctx, &vec![input.clone()]).unwrap();
        assert_eq!(encoded, expected_encoded, "encoded value mismatch");

        // Decode the result and verify we get back the original bytes
        let decoded = (decode_spec.runner)(&decode_spec, &auth_ctx, &vec![encoded]).unwrap();

        // Get expected bytes based on input type
        let expected_bytes = match &input {
            Value::Buffer(b) => b.clone(),
            Value::String(s) if s.starts_with("0x") => {
                txtx_addon_kit::hex::decode(&s[2..]).unwrap()
            }
            Value::String(s) => s.as_bytes().to_vec(),
            _ => unreachable!(),
        };
        assert_eq!(decoded, Value::buffer(expected_bytes), "decoded value mismatch");
    }

    #[test]
    fn test_decode_base64_invalid_input() {
        let fn_spec = get_spec_by_name("decode_base64");
        let args = vec![Value::string("!!!invalid!!!".to_string())];
        let result = (fn_spec.runner)(&fn_spec, &dummy_auth_ctx(), &args);
        assert!(result.is_err());
    }
}
