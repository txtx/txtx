use std::str::FromStr;

use anchor_lang_idl::types::Idl;
use solana_sdk::{pubkey::Pubkey, system_program};
use txtx_addon_kit::{
    indexmap::indexmap,
    types::{
        diagnostics::Diagnostic,
        functions::{
            arg_checker_with_ctx, fn_diag_with_ctx, FunctionImplementation, FunctionSpecification,
        },
        types::{ObjectProperty, ObjectType, Type, Value},
        AuthorizationContext,
    },
};

use crate::{
    codec::{anchor::AnchorProgramArtifacts, idl::IdlRef},
    constants::{DEFAULT_ANCHOR_TARGET_PATH, NAMESPACE},
    typing::{SvmValue, ANCHOR_PROGRAM_ARTIFACTS, PDA_RESULT, SVM_ADDRESS, SVM_IDL, SVM_PUBKEY},
};

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
                documentation: "`svm::system_program_id` returns the id of the system program, `11111111111111111111111111111111`.",
                example: indoc! {r#"
                    output "system_program_id" { 
                        value = svm::system_program_id()
                    }
                    // > 11111111111111111111111111111111
                "#},
                inputs: [
                ],
                output: {
                    documentation: "The system program id.",
                    typing: Type::addon(SVM_PUBKEY.into())
                },
            }
        },
        define_function! {
            DefaultPubkey => {
                name: "default_pubkey",
                documentation: "`svm::default_pubkey` returns a default public key, `11111111111111111111111111111111`.",
                example: indoc! {r#"
                    output "default_pubkey" { 
                        value = svm::default_pubkey()
                    }
                    // > 11111111111111111111111111111111
                "#},
                inputs: [
                ],
                output: {
                    documentation: "The default public key, `11111111111111111111111111111111`",
                    typing: Type::addon(SVM_PUBKEY.into())
                },
            }
        },
        define_function! {
            CreateAccountMeta => {
                name: "account",
                documentation: "`svm::account` encodes a public key in to an account meta object for a program instruction call.",
                example: indoc! {r#"
                    output "account" { 
                        value = svm::account("3z9vL1zjN6qyAFHhHQdWYRTFAcy69pJydkZmSFBKHg1R", true, true)
                    }
                    // > account: { public_key: 3z9vL1zjN6qyAFHhHQdWYRTFAcy69pJydkZmSFBKHg1R, is_signer: true, is_writable: true } 
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
                    documentation: "The account meta object.",
                    typing: Type::object(vec![ObjectProperty {
                        name: "public_key".into(),
                        documentation: "The public key of the account.".into(),
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false
                    },
                    ObjectProperty {
                        name: "is_signer".into(),
                        documentation: "Specifies if the account is a signer on the instruction.".into(),
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false
                    },
                    ObjectProperty {
                        name: "is_writable".into(),
                        documentation: "Specifies if the account is written to by the instruction.".into(),
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false
                    }])
                },
            }
        },
        define_function! {
            GetInstructionDataFromIdlPath => {
                name: "get_instruction_data_from_idl_path",
                documentation: "`svm::get_instruction_data_from_idl_path` creates encoded instruction data for a program invocation, providing type checking and serialization based on the provided IDL file.",
                example: indoc! {r#"
                    output "data" {
                        value = svm::get_instruction_data_from_idl("/path/to/idl.json", "my_instruction", ["arg1", "arg2"])
                    }
                    // > data: 0x95763bdcc47fa1b305000000776f726c64
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
                    documentation: "The encoded instruction data.",
                    typing: Type::buffer()
                },
            }
        },
        define_function! {
            GetInstructionDataFromIdl => {
                name: "get_instruction_data_from_idl",
                documentation: "`svm::get_instruction_data_from_idl_path` creates encoded instruction data for a program invocation, providing type checking and serialization based on the provided IDL data.",
                example: indoc! {r#"
                    output "data" {
                        value = svm::get_instruction_data_from_idl(variable.idl, "my_instruction", ["arg1", "arg2"])
                    }
                    // > data: 0x95763bdcc47fa1b305000000776f726c64
                "#},
                inputs: [
                    idl: {
                        documentation: "The program IDL.",
                        typing: vec![Type::addon(SVM_IDL), Type::string()],
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
                    documentation: "The encoded instruction data.",
                    typing: Type::buffer()
                },
            }
        },
        define_function! {
            GetProgramFromAnchorProject => {
                name: "get_program_from_anchor_project",
                documentation: "`svm::get_program_from_anchor_project` retrieves the program deployment artifacts for a program in an Anchor project.",
                example: indoc! {r#"
                    variable "contract" {
                        value = evm::get_program_from_anchor_project("my_program")
                    }
                    output "idl" {
                        value = variable.contract.idl
                    }    
                "#},
                inputs: [
                    program_name: {
                        documentation: "The name of the program being deployed.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    target_path: {
                        documentation: "The target path to the compiled anchor project artifacts. Defaults to `./target`.",
                        typing: vec![Type::string()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "An object containing the anchor program artifacts.",
                    typing: ANCHOR_PROGRAM_ARTIFACTS.clone()
                },
            }
        },
        define_function! {
            SolToLamports => {
                name: "sol_to_lamports",
                documentation: "`svm::sol_to_lamports` converts the provided SOL amount to lamports.",
                example: indoc! {r#"
                    output "lamports" {
                        value = svm::sol_to_lamports(1.1)
                    }
                    // lamports: 1100000000
                "#},
                inputs: [
                    sol_amount: {
                        documentation: "The amount of SOL to convert to lamports.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The amount of SOL provided, represented as lamports.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            LamportsToSol => {
                name: "lamports_to_sol",
                documentation: "`svm::lamports_to_sol` converts the provided number of lamports amount to SOL.",
                example: indoc! {r#"
                    output "sol" {
                        value = svm::lamports_to_sol(1100000000)
                    }
                    // sol: 1.1
                "#},
                inputs: [
                    lamports_amount: {
                        documentation: "The amount of lamports to convert to SOL.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The number of lamports provided, represented as SOL.",
                    typing: Type::float()
                },
            }
        },
        define_function! {
            FindPda => {
                name: "find_pda",
                documentation: "`svm::find_pda` finds a valid pda using the provided program id and seeds.",
                example: indoc! {r#"
                    variable "pda" {
                        value = svm::find_pda("3bv3j4GvMPjvvBX9QdoX27pVoWhDSXpwKZipFF1QiVr6", ["data"])
                    }
                    output "pda" {
                        value = std::encode_base58(variable.pda.pda)
                    }
                    output "bump" {
                        value = variable.pda.bump_seed
                    }
                    // > pda: 4amHoWMBgLkPfM8Nq9ZP33Liq9FCuqrLoU1feejkdsUJ
                    // > bump: 252
                "#},
                inputs: [
                    program_id: {
                        documentation: "The address of the program the PDA is derived from.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: false
                    },
                    seeds: {
                        documentation: "An optional array of seeds that will be used to derive the PDA.",
                        typing: vec![Type::array(Type::string())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "An object containing the PDA address and associated bump seed.",
                    typing: PDA_RESULT.clone()
                },
            }
        }
    ];
}

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
        arg_checker(fn_spec, args)?;
        Ok(SvmValue::pubkey(system_program::id().to_bytes().to_vec()))
    }
}
pub struct DefaultPubkey;
impl FunctionImplementation for DefaultPubkey {
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
        Ok(SvmValue::pubkey(Pubkey::default().to_bytes().to_vec()))
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

pub struct GetInstructionDataFromIdl;
impl FunctionImplementation for GetInstructionDataFromIdl {
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
        // let idl_bytes = &args.get(0).unwrap().as_addon_data().unwrap().bytes;
        let idl_str = args.get(0).unwrap().as_string().unwrap();
        let instruction_name = args.get(1).unwrap().as_string().unwrap();
        let arguments =
            args.get(2).and_then(|a| Some(a.as_array().unwrap().to_vec())).unwrap_or(vec![]);

        // let idl: Idl = serde_json::from_slice(&idl_bytes)
        //     .map_err(|e| to_diag(fn_spec, format!("invalid idl: {e}")))?;
        let idl: Idl = serde_json::from_str(idl_str)
            .map_err(|e| to_diag(fn_spec, format!("invalid idl: {e}")))?;

        let idl_ref = IdlRef::from_idl(idl);

        let mut data =
            idl_ref.get_discriminator(&instruction_name).map_err(|e| to_diag(fn_spec, e))?;
        let mut encoded_args = idl_ref
            .get_encoded_args(&instruction_name, arguments)
            .map_err(|e| to_diag(fn_spec, e))?;
        data.append(&mut encoded_args);

        Ok(Value::buffer(data))
    }
}

pub struct GetInstructionDataFromIdlPath;
impl FunctionImplementation for GetInstructionDataFromIdlPath {
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

        let idl_path = auth_ctx
            .get_path_from_str(idl_path_str)
            .map_err(|e| to_diag(fn_spec, format!("failed to get idl: {e}")))?;

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

pub struct GetProgramFromAnchorProject;
impl FunctionImplementation for GetProgramFromAnchorProject {
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
        let program_name = args.get(0).unwrap().as_string().unwrap();
        let target_path_str =
            args.get(1).and_then(|v| v.as_string()).unwrap_or(DEFAULT_ANCHOR_TARGET_PATH);

        let target_path = auth_ctx
            .get_path_from_str(target_path_str)
            .map_err(|e| to_diag(fn_spec, format!("failed to get anchor target path: {e}")))?;

        let anchor_program_artifacts =
            AnchorProgramArtifacts::new(target_path.expect_path_buf(), &program_name)
                .map_err(|e| to_diag(fn_spec, e))?;

        let value = anchor_program_artifacts.to_value().map_err(|e| to_diag(fn_spec, e))?;
        Ok(value)
    }
}

pub struct SolToLamports;
impl FunctionImplementation for SolToLamports {
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
        let sol = args.get(0).unwrap();
        let sol = match sol {
            Value::Integer(i) => {
                if *i < 0 {
                    return Err(to_diag(fn_spec, "SOL amount cannot be negative".into()));
                }
                if *i > (1u64 << 53) as i128 {
                    return Err(to_diag(
                        fn_spec,
                        "SOL amount too large for precise conversion".into(),
                    ));
                }
                *i as f64
            }
            Value::Float(f) => {
                if *f < 0.0 {
                    return Err(to_diag(fn_spec, "SOL amount cannot be negative".into()));
                }
                *f
            }
            _ => unreachable!(),
        };
        let lamports = solana_sdk::native_token::sol_to_lamports(sol);
        Ok(Value::integer(lamports as i128))
    }
}

pub struct LamportsToSol;
impl FunctionImplementation for LamportsToSol {
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
        let lamports = args.get(0).unwrap().as_uint().unwrap().map_err(|e| to_diag(fn_spec, e))?;

        let sol = solana_sdk::native_token::lamports_to_sol(lamports);
        Ok(Value::float(sol))
    }
}

pub struct FindPda;
impl FunctionImplementation for FindPda {
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
        let program_id = SvmValue::to_pubkey(args.get(0).unwrap())
            .map_err(|e| to_diag(fn_spec, format!("invalid program id for finding pda: {e}")))?;

        let seeds: Vec<Vec<u8>> = args
            .get(1)
            .map(|v| {
                v.as_array()
                    .ok_or_else(|| to_diag(fn_spec, "seeds must be an array".to_string()))?
                    .iter()
                    .map(|s| {
                        let bytes = s.to_bytes();
                        if bytes.is_empty() {
                            return Err(to_diag(fn_spec, "seed cannot be empty".to_string()));
                        }
                        Ok(bytes)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        let seed_refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::try_find_program_address(&seed_refs, &program_id)
            .ok_or(to_diag(fn_spec, "failed to find pda".to_string()))?;
        let obj = ObjectType::from(vec![
            ("pda", SvmValue::pubkey(pda.to_bytes().to_vec())),
            ("bump_seed", Value::integer(bump as i128)),
        ])
        .to_value();
        Ok(obj)
    }
}
