use std::path::PathBuf;

use crate::{codec::utils::get_seeds_from_value, typing::anchor::types::Idl};

use crate::constants::{DEFAULT_NATIVE_TARGET_PATH, DEFAULT_SHANK_IDL_PATH};
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{
        arg_checker_with_ctx, fn_diag_with_ctx, FunctionImplementation, FunctionSpecification,
    },
    namespace::Namespace,
    types::{ObjectType, Type, Value},
    AuthorizationContext,
};

use crate::{
    codec::{anchor::AnchorProgramArtifacts, idl::IdlRef, native::NativeProgramArtifacts},
    constants::{DEFAULT_ANCHOR_TARGET_PATH, NAMESPACE},
    typing::{
        SvmValue, ANCHOR_PROGRAM_ARTIFACTS, CLASSIC_RUST_PROGRAM_ARTIFACTS, PDA_RESULT, SVM_IDL,
        SVM_PUBKEY,
    },
};

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

const LAMPORTS_PER_SOL_F64: f64 = LAMPORTS_PER_SOL as f64;

pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL_F64).round() as u64
}

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL_F64
}

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &[Value]) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(Namespace::from(NAMESPACE));
    checker(fn_spec, args)
}
pub fn to_diag<T: ToString>(fn_spec: &FunctionSpecification, e: T) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(Namespace::from(NAMESPACE));
    error_fn(fn_spec, e.to_string())
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
                        value = svm::get_program_from_anchor_project("my_program")
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
                    keypair_path: {
                        documentation: "The location of the program keypair file. Defaults to `./target/deploy/<program_name>-keypair.json`.",
                        typing: vec![Type::string(), Type::null()],
                        optional: true
                    },
                    idl_path: {
                        documentation: "The location of the program IDL file. Defaults to `./target/idl/<program_name>.json`.",
                        typing: vec![Type::string(), Type::null()],
                        optional: true
                    },
                    bin_path: {
                        documentation: "The location of the program binary file. Defaults to `./target/deploy/<program_name>.so`.",
                        typing: vec![Type::string(), Type::null()],
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
            GetProgramFromNativeProject => {
                name: "get_program_from_native_project",
                documentation: "`svm::get_program_from_native_project` retrieves the program deployment artifacts for a non-Anchor program.",
                example: indoc! {r#"
                    variable "contract" {
                        value = svm::get_program_from_native_project("my_program")
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
                    keypair_path: {
                        documentation: "The location of the program keypair file. Defaults to `./target/deploy/<program_name>-keypair.json`.",
                        typing: vec![Type::string(), Type::null()],
                        optional: true
                    },
                    idl_path: {
                        documentation: "The location of the program IDL file. Defaults to `./idl/<program_name>.json`.",
                        typing: vec![Type::string(), Type::null()],
                        optional: true
                    },
                    bin_path: {
                        documentation: "The location of the program binary file. Defaults to `./target/deploy/<program_name>.so`.",
                        typing: vec![Type::string(), Type::null()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "An object containing the native program artifacts.",
                    typing: CLASSIC_RUST_PROGRAM_ARTIFACTS.clone()
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
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: false
                    },
                    seeds: {
                        documentation: "An optional array of seeds that will be used to derive the PDA. A maximum of 16 seeds can be used, and each seed can have a maximum length of 32 bytes.",
                        typing: vec![Type::array(Type::string())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "An object containing the PDA address and associated bump seed.",
                    typing: PDA_RESULT.clone()
                },
            }
        },
        define_function! {
            GetAssociatedTokenAccount => {
                name: "get_associated_token_account",
                documentation: "`svm::get_associated_token_account` computes the address of the associated token account for the provided wallet and token mint addresses.",
                example: indoc! {r#"
                    variable "token_account" {
                        value = svm::get_associated_token_account(signer.caller.address, "So11111111111111111111111111111111111111112")
                    }
                "#},
                inputs: [
                    wallet_address: {
                        documentation: "The address of the wallet to compute the associated token account for.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: false
                    },
                    token_mint_address: {
                        documentation: "The address of the token mint used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The address of the associated token account.",
                    typing: Type::addon(SVM_PUBKEY.into())
                },
            }
        },
        define_function! {
            CreateTokenAccountInstruction => {
                name: "create_token_account_instruction",
                documentation: "`svm::create_token_account_instruction` creates raw instruction bytes to create an associated token account.",
                example: indoc! {r#"
                    action "call" "svm::process_instructions" {
                        signers = [signer.caller]

                        instruction { 
                            raw_bytes = svm::create_token_account_instruction(
                                signer.caller.address, // funding address
                                signer.caller.address, // wallet address
                                variable.token_mint, // token mint address
                                variable.token_program // token program id
                            )
                        }
                    }
                "#},
                inputs: [
                    funding_address: {
                        documentation: "The address of the funding account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: false
                    },
                    wallet_address: {
                        documentation: "The address of the wallet to compute the associated token account for.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: false
                    },
                    token_mint_address: {
                        documentation: "The address of the token mint used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: true
                    },
                    token_program_id: {
                        documentation: "The address of the token program used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The serialized instruction bytes.",
                    typing: Type::addon(SVM_PUBKEY.into())
                },
            }
        },
        define_function! {
            SvmU64 => {
                name: "u64",
                documentation: "`svm::u64` creates a byte array representation of a u64 integer, suitable for use as a seed in PDA derivation.",
                example: indoc! {r#"
                    variable "u64" {
                        value = svm::u64(1000000000)
                    }
                "#},
                inputs: [
                    value: {
                        documentation: "The u64 integer to convert to a byte array.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The byte array representation of the provided u64 integer.",
                    typing: Type::buffer()
                },
            }
        },
        define_function! {
            SvmI64 => {
                name: "i64",
                documentation: "`svm::i64` creates a byte array representation of a i64 integer, suitable for use as a seed in PDA derivation.",
                example: indoc! {r#"
                    variable "i64" {
                        value = svm::i64(-1000000000)
                    }
                "#},
                inputs: [
                    value: {
                        documentation: "The i64 integer to convert to a byte array.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The byte array representation of the provided i64 integer.",
                    typing: Type::buffer()
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
        Ok(SvmValue::pubkey(system_program::id().to_bytes().to_vec()))
    }
}
pub struct DefaultPubkey;
impl FunctionImplementation for DefaultPubkey {
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
        Ok(SvmValue::pubkey(Pubkey::default().to_bytes().to_vec()))
    }
}
pub struct GetInstructionDataFromIdl;
impl FunctionImplementation for GetInstructionDataFromIdl {
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
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let idl_path_str = args.get(0).unwrap().as_string().unwrap();
        let instruction_name = args.get(1).unwrap().as_string().unwrap();
        let arguments =
            args.get(2).and_then(|a| Some(a.as_array().unwrap().to_vec())).unwrap_or(vec![]);

        let idl_path = auth_ctx
            .get_file_location_from_path_buf(&PathBuf::from(idl_path_str))
            .map_err(|e| to_diag(fn_spec, format!("failed to get idl: {e}")))?;

        let idl_ref = IdlRef::from_location(idl_path).map_err(|e| to_diag(fn_spec, e))?;
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
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let program_name = args.get(0).unwrap().as_string().unwrap();

        let keypair_path_buf = match args.get(1) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_ANCHOR_TARGET_PATH)
                .join("deploy")
                .join(format!("{}-keypair.json", program_name)),
            Some(Value::String(s)) => PathBuf::from(s),
            _ => unreachable!(),
        };

        let keypair_path =
            auth_ctx.get_file_location_from_path_buf(&keypair_path_buf).map_err(|e| {
                to_diag(fn_spec, format!("failed to get anchor program keypair path: {e}"))
            })?;

        let idl_path_buf = match args.get(2) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_ANCHOR_TARGET_PATH)
                .join("idl")
                .join(format!("{}.json", program_name)),
            Some(Value::String(s)) => PathBuf::from(s),
            _ => unreachable!(),
        };

        let idl_path = auth_ctx
            .get_file_location_from_path_buf(&idl_path_buf)
            .map_err(|e| to_diag(fn_spec, format!("failed to get anchor program idl path: {e}")))?;

        let bin_path_buf = match args.get(3) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_ANCHOR_TARGET_PATH)
                .join("deploy")
                .join(format!("{}.so", program_name)),
            Some(Value::String(s)) => PathBuf::from(s),
            _ => unreachable!(),
        };

        let bin_path = auth_ctx.get_file_location_from_path_buf(&bin_path_buf).map_err(|e| {
            to_diag(fn_spec, format!("failed to get anchor program binary path: {e}"))
        })?;

        let anchor_program_artifacts = AnchorProgramArtifacts::new(
            keypair_path.expect_path_buf(),
            idl_path.expect_path_buf(),
            bin_path.expect_path_buf(),
        )
        .map_err(|e| to_diag(fn_spec, e))?;

        let value = anchor_program_artifacts.to_value().map_err(|e| to_diag(fn_spec, e))?;
        Ok(value)
    }
}

pub struct GetProgramFromNativeProject;
impl FunctionImplementation for GetProgramFromNativeProject {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let program_name = args.get(0).unwrap().as_string().unwrap();

        let keypair_path_buf = match args.get(1) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_NATIVE_TARGET_PATH)
                .join("deploy")
                .join(format!("{}-keypair.json", program_name)),
            Some(Value::String(s)) => PathBuf::from(s),
            _ => unreachable!(),
        };

        let keypair_path = auth_ctx
            .get_file_location_from_path_buf(&keypair_path_buf)
            .map_err(|e| to_diag(fn_spec, format!("failed to get program keypair path: {e}")))?;

        let mut did_user_provide_idl_path = false;

        let idl_path_buf = match args.get(2) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_SHANK_IDL_PATH)
                .join("idl")
                .join(format!("{}.json", program_name)),
            Some(Value::String(s)) => {
                did_user_provide_idl_path = true;
                PathBuf::from(s)
            }
            _ => unreachable!(),
        };

        let idl_path = auth_ctx
            .get_file_location_from_path_buf(&idl_path_buf)
            .map_err(|e| to_diag(fn_spec, format!("failed to get shank idl path: {e}")))?;

        if did_user_provide_idl_path && !idl_path.exists() {
            return Err(to_diag(
                fn_spec,
                format!("invalid program idl path; no idl found at: {}", idl_path_buf.display()),
            ));
        }

        let bin_path_buf = match args.get(3) {
            Some(Value::Null) | None => PathBuf::from(DEFAULT_NATIVE_TARGET_PATH)
                .join("deploy")
                .join(format!("{}.so", program_name)),
            Some(Value::String(s)) => PathBuf::from(s),
            _ => unreachable!(),
        };

        let bin_path = auth_ctx
            .get_file_location_from_path_buf(&bin_path_buf)
            .map_err(|e| to_diag(fn_spec, format!("failed to get program binary path: {e}")))?;

        let classic_program_artifacts =
            NativeProgramArtifacts::new(keypair_path, idl_path, bin_path)
                .map_err(|e| to_diag(fn_spec, e.message))?;

        classic_program_artifacts.to_value()
    }
}

pub struct SolToLamports;
impl FunctionImplementation for SolToLamports {
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
        let sol = args.get(0).unwrap();
        let sol = match sol {
            Value::Integer(i) => {
                if *i < 0 {
                    return Err(to_diag(fn_spec, "SOL amount cannot be negative"));
                }
                if *i > (1u64 << 53) as i128 {
                    return Err(to_diag(fn_spec, "SOL amount too large for precise conversion"));
                }
                *i as f64
            }
            Value::Float(f) => {
                if *f < 0.0 {
                    return Err(to_diag(fn_spec, "SOL amount cannot be negative"));
                }
                *f
            }
            _ => unreachable!(),
        };
        let lamports = sol_to_lamports(sol);
        Ok(Value::integer(lamports as i128))
    }
}

pub struct LamportsToSol;
impl FunctionImplementation for LamportsToSol {
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
        let lamports = args.get(0).unwrap().as_uint().unwrap().map_err(|e| to_diag(fn_spec, e))?;

        let sol = lamports_to_sol(lamports);
        Ok(Value::float(sol))
    }
}

pub struct FindPda;
impl FunctionImplementation for FindPda {
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
        let program_id = SvmValue::to_pubkey(args.get(0).unwrap())
            .map_err(|e| to_diag(fn_spec, format!("invalid program id for finding pda: {e}")))?;

        let seeds = if let Some(val) = args.get(1) {
            get_seeds_from_value(val).map_err(|diag| to_diag(fn_spec, diag))?
        } else {
            vec![]
        };

        let seed_refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::try_find_program_address(&seed_refs, &program_id)
            .ok_or(to_diag(fn_spec, "failed to find pda".to_string()))?;
        let obj = ObjectType::from(vec![
            ("pda", Value::string(pda.to_string())),
            ("bump_seed", Value::integer(bump as i128)),
        ])
        .to_value();
        Ok(obj)
    }
}

pub struct GetAssociatedTokenAccount;
impl FunctionImplementation for GetAssociatedTokenAccount {
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
        let wallet_address = SvmValue::to_pubkey(args.get(0).unwrap()).map_err(|e| {
            to_diag(
                fn_spec,
                format!("invalid wallet address for getting associated token account: {e}"),
            )
        })?;

        let token_mint_address = SvmValue::to_pubkey(args.get(1).unwrap()).map_err(|e| {
            to_diag(
                fn_spec,
                format!("invalid token mint address for getting associated token account: {e}"),
            )
        })?;

        let spl_associated_token_account =
            spl_associated_token_account_interface::address::get_associated_token_address(
                &wallet_address,
                &token_mint_address,
            );

        Ok(SvmValue::pubkey(spl_associated_token_account.to_bytes().to_vec()))
    }
}

pub struct CreateTokenAccountInstruction;
impl FunctionImplementation for CreateTokenAccountInstruction {
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
        let funding_address = SvmValue::to_pubkey(args.get(0).unwrap()).map_err(|e| {
            to_diag(fn_spec, format!("invalid funding address for creating token account: {e}"))
        })?;

        let wallet_address = SvmValue::to_pubkey(args.get(1).unwrap()).map_err(|e| {
            to_diag(fn_spec, format!("invalid wallet address for creating token account: {e}"))
        })?;

        let token_mint_address = SvmValue::to_pubkey(args.get(2).unwrap()).map_err(|e| {
            to_diag(fn_spec, format!("invalid token mint address for creating token account: {e}"))
        })?;

        let token_program_id = SvmValue::to_pubkey(args.get(3).unwrap()).map_err(|e| {
            to_diag(fn_spec, format!("invalid token program id for creating token account: {e}"))
        })?;

        let instruction = create_associated_token_account_idempotent(
            &funding_address,
            &wallet_address,
            &token_mint_address,
            &token_program_id,
        );
        let bytes = serde_json::to_vec(&instruction).map_err(|e| {
            to_diag(fn_spec, format!("failed to serialize create token account instruction: {e}"))
        })?;

        Ok(SvmValue::instruction(bytes))
    }
}

pub struct SvmU64;
impl FunctionImplementation for SvmU64 {
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
        let value = args.get(0).unwrap().as_uint().unwrap().map_err(|e| to_diag(fn_spec, e))?;
        Ok(SvmValue::u64(value))
    }
}
pub struct SvmI64;
impl FunctionImplementation for SvmI64 {
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
        let value = args.get(0).unwrap().as_integer().unwrap();
        let value: i64 =
            value.try_into().map_err(|_| to_diag(fn_spec, "i64 value out of range"))?;
        Ok(SvmValue::i64(value))
    }
}
