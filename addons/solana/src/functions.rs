use std::path::Path;

use solana_sdk::system_program;
use txtx_addon_kit::{
    helpers::fs::FileLocation,
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

use crate::{codec::idl::IdlRef, constants::NAMESPACE, typing::SOLANA_ACCOUNT};

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(NAMESPACE.to_string());
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(NAMESPACE.to_string());
    error_fn(fn_spec, e)
}

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            SystemProgramId => {
                name: "system_program_id",
                documentation: "`solana::system_program_id` returns the id of the system program, `11111111111111111111111111111111`.",
                example: indoc! {r#"
                    output "system_program_id" { 
                        value = solana::system_program_id()
                    }
                    // > 
                "#},
                inputs: [
                ],
                output: {
                    documentation: "The system program id",
                    typing: Type::addon(SOLANA_PUBKEY.into())
                },
            }
        },
        define_function! {
            CreateAccountMeta => {
                name: "account",
                documentation: "`solana::account` is coming soon",
                example: indoc! {r#"
                    output "account" { 
                    value = solana::account("3z9vL1zjN6qyAFHhHQdWYRTFAcy69pJydkZmSFBKHg1R", true, true)
                    }
                    // > 
                "#},
                inputs: [
                    public_key: {
                        documentation: "The on-chain address of an account.",
                        typing: vec![Type::string()]
                    },
                    is_signer: {
                        documentation: "Specify if the account is required as a signer on the transaction.",
                        typing: vec![Type::bool()]
                    },
                    is_writable: {
                        documentation: "Specify if the account data will be modified.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "Coming soon.",
                    typing: Type::addon(SOLANA_ACCOUNT.into())
                },
            }
        },
        define_function! {
            GetDataFromAnchorProject => {
                name: "get_instruction_data_from_idl",
                documentation: "`solana::get_instruction_data_from_idl` is coming soon",
                example: indoc! {r#"
                    // Coming soon
                "#},
                inputs: [
                    idl_path: {
                        documentation: "The path, relative to the txtx.yml, to the IDL `.json` file.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    instruction_name: {
                        documentation: "The name of the instruction to generate data for, as indexed by the IDL.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    arguments: {
                        documentation: "The instruction arguments to generate data for.",
                        typing: vec![Type::array(Type::string())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "Coming soon.",
                    typing: Type::addon(SOLANA_ACCOUNT.into())
                },
            }
        }
    ];

pub struct SystemProgramId;
impl FunctionImplementation for SystemProgramId {
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
        Ok(Value::string(system_program::id().to_string()))
    }
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

pub struct GetDataFromAnchorProject;
impl FunctionImplementation for GetDataFromAnchorProject {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let idl_path_str = args.get(0).unwrap().as_string().unwrap();
        let instruction_name = args.get(1).unwrap().as_string().unwrap();
        let arguments =
            args.get(2).and_then(|a| Some(a.as_array().unwrap().to_vec())).unwrap_or(vec![]);

        let idl_path = Path::new(&idl_path_str);
        let idl_path = if idl_path.is_absolute() {
            FileLocation::from_path(idl_path.to_path_buf())
        } else {
            let mut workspace_loc = auth_ctx
                .workspace_location
                .get_parent_location()
                .map_err(|e| to_diag(fn_spec, format!("unable to read workspace location: {e}")))?;

            workspace_loc
                .append_path(&idl_path_str.to_string())
                .map_err(|e| to_diag(fn_spec, format!("invalid hardhat config path: {}", e)))?;
            workspace_loc
        };

        let idl_ref = IdlRef::new(idl_path).map_err(|e| to_diag(fn_spec, e))?;
        let mut data =
            idl_ref.get_discriminator(&instruction_name).map_err(|e| to_diag(fn_spec, e))?;
        let mut encoded_args = idl_ref
            .get_encoded_args(&instruction_name, arguments)
            .map_err(|e| to_diag(fn_spec, e))?;
        data.append(&mut encoded_args);

        Ok(Value::buffer(data))
    }
}
