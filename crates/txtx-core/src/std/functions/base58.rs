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
                documentation: "`encode_base58` encodes a buffer or hex string as a base58 string.",
                example: indoc!{r#"
                output "encoded" {
                    value = encode_base58("0xaca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8")
                }
                > encoded: CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF
              "#},
                inputs: [
                    value: {
                        documentation: "The buffer or hex string to encode.",
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
        let bytes = args
            .get(0)
            .unwrap()
            .get_buffer_bytes_result()
            .map_err(|e| to_diag(fn_spec, e))?;

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
