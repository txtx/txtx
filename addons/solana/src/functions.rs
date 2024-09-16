use txtx_addon_kit::{
    indexmap::indexmap,
    types::{
        diagnostics::Diagnostic,
        functions::{
            arg_checker_with_ctx, fn_diag_with_ctx, FunctionImplementation, FunctionSpecification,
        },
        types::{Type, Value},
        AuthorizationContext,
    },
};

use crate::{constants::NAMESPACE, typing::SOLANA_ACCOUNT};

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(NAMESPACE.to_string());
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(NAMESPACE.to_string());
    error_fn(fn_spec, e)
}

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        CreateAccountMeta => {
            name: "account",
            documentation: "`solana::account` wraps the given Clarity value in a Clarity `Optional`.",
            example: indoc! {r#"
                output "some" { 
                  value = solana::account("3z9vL1zjN6qyAFHhHQdWYRTFAcy69pJydkZmSFBKHg1R", true, true)
                }
                // > 
                "#},
            inputs: [
                public_key: {
                    documentation: "The on-chain address of an account",
                    typing: vec![Type::string()]
                },
                is_signer: {
                    documentation: "Specify if the account is required as a signer on the transaction",
                    typing: vec![Type::bool()]
                },
                is_writable: {
                    documentation: "Specify if the account data will be modified",
                    typing: vec![Type::bool()]
                }
            ],
            output: {
                documentation: "The input Clarity value wrapped in a Clarity `Optional`.",
                typing: Type::addon(SOLANA_ACCOUNT.into())
            },
        }
    }];
}

pub struct CreateAccountMeta;
impl FunctionImplementation for CreateAccountMeta {
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
        let public_key = args.get(0).unwrap();
        let is_signer = args.get(1).unwrap();
        let is_writable = args.get(2).unwrap();

        Ok(Value::object(indexmap! {
            "public_key".to_string() => public_key.clone(),
            "is_signer".to_string() => is_signer.clone(),
            "is_writable".to_string() => is_writable.clone()
        }))
    }
}
