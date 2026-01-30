use super::{arg_checker, to_diag};
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};
use txtx_addon_kit::types::AuthorizationContext;

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            Base58Encode => {
                name: "encode_base58",
                documentation: "`encode_base58` encodes a buffer or string as a base58 string.",
                example: indoc!{r#"
                output "encoded" {
                    value = encode_base58("0xaca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8")
                }
                > encoded: CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF
              "#},
                inputs: [
                    value: {
                        documentation: "The buffer or string to encode. Strings starting with '0x' are decoded as hex; otherwise, raw UTF-8 bytes are used.",
                        typing: vec![Type::buffer(), Type::string(), Type::addon("any")]
                    }
                ],
                output: {
                    documentation: "The input, encoded as a base58 string.",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            Base58Decode => {
                name: "decode_base58",
                documentation: "`decode_base58` decodes a base58 encoded string and returns the result as a buffer.",
                example: indoc!{r#"
                output "decoded" {
                    value = decode_base58("CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF")
                }
                > decoded: 0xaca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8
              "#},
                inputs: [
                    base58_string: {
                        documentation: "The base58 string to decode.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The decoded base58 string, as a buffer.",
                    typing: Type::buffer()
                },
            }
        },
    ];
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

pub struct Base58Encode;
impl FunctionImplementation for Base58Encode {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let bytes = get_bytes_for_encoding(args.get(0).unwrap())?;
        let encoded = bs58::encode(bytes).into_string();
        Ok(Value::string(encoded))
    }
}

pub struct Base58Decode;
impl FunctionImplementation for Base58Decode {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let encoded = args.get(0).unwrap().expect_string();

        let decoded = bs58::decode(encoded)
            .into_vec()
            .map_err(|e| to_diag(fn_spec, format!("failed to decode base58 string {}: {}", encoded, e)))?;

        Ok(Value::buffer(decoded))
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;
    use txtx_addon_kit::helpers::fs::FileLocation;
    use txtx_addon_kit::hex as kit_hex;

    use super::*;

    fn get_spec_by_name(name: &str) -> FunctionSpecification {
        FUNCTIONS.iter().find(|f| f.name == name).cloned().unwrap()
    }

    fn dummy_auth_ctx() -> AuthorizationContext {
        AuthorizationContext { workspace_location: FileLocation::working_dir() }
    }

    fn hex_to_buffer(hex: &str) -> Value {
        Value::buffer(kit_hex::decode(hex).unwrap())
    }

    #[test_case(
        hex_to_buffer("aca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8"),
        Value::string("CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF".to_string());
        "buffer 32 bytes"
    )]
    #[test_case(
        Value::string("0xaca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8".to_string()),
        Value::string("CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF".to_string());
        "hex string 32 bytes"
    )]
    #[test_case(
        Value::string("hello".to_string()),
        Value::string("Cn8eVZg".to_string());
        "plain string hello"
    )]
    #[test_case(
        Value::string("__event_authority".to_string()),
        Value::string("tyvCZETMWX6hYsUwTchxRWG".to_string());
        "plain string with underscores"
    )]
    #[test_case(
        Value::buffer(vec![0]),
        Value::string("1".to_string());
        "single zero byte"
    )]
    #[test_case(
        Value::buffer(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        Value::string("8DfbjXLth7APvt3qQPgtf".to_string());
        "sequential bytes"
    )]
    #[test_case(
        Value::buffer(vec![0, 0, 0, 1]),
        Value::string("1112".to_string());
        "leading zeros preserved"
    )]
    fn test_base58_encode_decode_roundtrip(input: Value, expected_encoded: Value) {
        let encode_spec = get_spec_by_name("encode_base58");
        let decode_spec = get_spec_by_name("decode_base58");
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
                kit_hex::decode(&s[2..]).unwrap()
            }
            Value::String(s) => s.as_bytes().to_vec(),
            _ => unreachable!(),
        };
        assert_eq!(decoded, Value::buffer(expected_bytes), "decoded value mismatch");
    }

    #[test]
    fn test_decode_base58_invalid_input() {
        let fn_spec = get_spec_by_name("decode_base58");
        // "0", "O", "I", "l" are not valid in base58
        let args = vec![Value::string("0OIl".to_string())];
        let result = (fn_spec.runner)(&fn_spec, &dummy_auth_ctx(), &args);
        assert!(result.is_err());
    }
}
