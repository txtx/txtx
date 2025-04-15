use std::str::FromStr;

use crate::typing::anchor::types::Idl;
use solana_sdk::{pubkey::Pubkey, system_program};
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{
        arg_checker_with_ctx, fn_diag_with_ctx, FunctionImplementation, FunctionSpecification,
    },
    types::{ObjectType, Type, Value},
    AuthorizationContext,
};

use crate::{
    codec::{anchor::AnchorProgramArtifacts, idl::IdlRef, native::ClassicRustProgramArtifacts},
    constants::{DEFAULT_ANCHOR_TARGET_PATH, NAMESPACE},
    typing::{
        SvmValue, ANCHOR_PROGRAM_ARTIFACTS, CLASSIC_RUST_PROGRAM_ARTIFACTS, PDA_RESULT,
        SVM_ADDRESS, SVM_IDL, SVM_PUBKEY,
    },
};

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(NAMESPACE.to_string());
    checker(fn_spec, args)
}
pub fn to_diag<T: ToString>(fn_spec: &FunctionSpecification, e: T) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(NAMESPACE.to_string());
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
            GetProgramFromNativeProject => {
                name: "get_program_from_native_project",
                documentation: "`svm::get_program_from_native_project` retrieves the program deployment artifacts for a program in a classic Rust project.",
                example: indoc! {r#"
                    variable "contract" {
                        value = svm::get_program_from_native_project("./bin/loc", "./keypair/loc")
                    }
                "#},
                inputs: [
                    binary_location: {
                        documentation: "The path, relative to the txtx.yml, to the compiled program binary.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    program_keypair_path: {
                        documentation: "The path, relative to the txtx.yml, to the program keypair.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "An object containing the rust program artifacts.",
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
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
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
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: false
                    },
                    token_mint_address: {
                        documentation: "The address of the token mint used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
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
                documentation: "`svm::create_token_account_instruction` creates an instruction to create an associated token account.",
                example: indoc! {r#"
                    action "call" "svm::process_instructions" {
                        signers = [signer.caller]

                        instruction { 
                            value = svm::create_token_account_instruction(
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
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: false
                    },
                    wallet_address: {
                        documentation: "The address of the wallet to compute the associated token account for.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: false
                    },
                    token_mint_address: {
                        documentation: "The address of the token mint used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: true
                    },
                    token_program_id: {
                        documentation: "The address of the token program used to compute the token account.",
                        typing: vec![Type::string(), Type::addon(SVM_PUBKEY.into()), Type::addon(SVM_ADDRESS.into())],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The serialized instruction bytes.",
                    typing: Type::addon(SVM_PUBKEY.into())
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

pub struct GetProgramFromNativeProject;
impl FunctionImplementation for GetProgramFromNativeProject {
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
        let bin_path = args.get(0).unwrap().as_string().unwrap();
        let keypair_path = args.get(1).unwrap().as_string().unwrap();

        let bin_path = auth_ctx
            .get_path_from_str(bin_path)
            .map_err(|e| to_diag(fn_spec, format!("failed to get program binary path: {e}")))?;

        let keypair_path = auth_ctx
            .get_path_from_str(keypair_path)
            .map_err(|e| to_diag(fn_spec, format!("failed to get program keypair path: {e}")))?;

        let classic_program_artifacts = ClassicRustProgramArtifacts::new(bin_path, keypair_path)
            .map_err(|e| to_diag(fn_spec, e.message))?;

        let value = classic_program_artifacts.to_value();
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
                        if bytes.len() > 32 {
                            if let Ok(pubkey) = Pubkey::from_str(&s.to_string()) {
                                return Ok(pubkey.to_bytes().to_vec());
                            } else {
                                return Err(to_diag(
                                    fn_spec,
                                    "seed cannot be longer than 32 bytes".to_string(),
                                ));
                            }
                        }
                        Ok(bytes)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        if seeds.len() > 16 {
            return Err(to_diag(fn_spec, "seeds a maximum of 16 seeds can be used".to_string()));
        }

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

pub struct GetAssociatedTokenAccount;
impl FunctionImplementation for GetAssociatedTokenAccount {
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
            spl_associated_token_account::get_associated_token_address(
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
