use super::{arg_checker, to_diag};
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};
use txtx_addon_kit::{hex, types::AuthorizationContext};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        Base58Encode => {
            name: "encode_base58",
            documentation: "`encode_base58` encodes a hex string as a base58 string.",
            example: indoc!{r#"
            output "encoded" { 
                value = encode_base58("0xaca1e2ae0c54a9a8f12da5dde27a93bb5ff94aeef722b1e474a16318234f83c8")
            }
            > encoded: CctJBuDbaFtojUWfQ3iEcq77eFDjojCtoS4Q59f6bUtF
          "#},
            inputs: [
                hex_string: {
                    documentation: "The hex string to encode.",
                    typing: vec![Type::string(), Type::addon("any")]
                }
            ],
            output: {
                documentation: "The hex string, encoded as a base58 string.",
                typing: Type::string()
            },
        }
    },];
}

pub struct Base58Encode;
impl FunctionImplementation for Base58Encode {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let hex = args.get(0).unwrap().to_string();
        let hex = if hex.starts_with("0x") { &hex[2..] } else { &hex[..] };

        let bytes = hex::decode(hex)
            .map_err(|e| to_diag(fn_spec, format!("failed to decode hex string {}: {}", hex, e)))?;

        let encoded = bs58::encode(bytes).into_string();
        Ok(Value::string(format!("{}", encoded)))
    }
}
