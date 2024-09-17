use std::{path::Path, str::FromStr};

use alloy::{
    dyn_abi::DynSolValue,
    hex::FromHex,
    primitives::{Bytes, B256},
};
use alloy_chains::ChainKind;
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    indexmap,
    types::functions::{arg_checker_with_ctx, fn_diag_with_ctx},
};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{ObjectType, Type, Value},
        AuthorizationContext,
    },
};

use crate::{
    codec::{
        foundry::FoundryConfig, generate_create2_address, hardhat::HardhatBuildArtifacts,
        string_to_address, value_to_sol_value,
    },
    commands::actions::deploy_contract::create_init_code,
    constants::{DEFAULT_CREATE2_FACTORY_ADDRESS, NAMESPACE},
    typing::{
        EvmValue, CHAIN_DEFAULTS, DEPLOYMENT_ARTIFACTS_TYPE, EVM_ADDRESS, EVM_BYTES, EVM_BYTES32,
        EVM_INIT_CODE,
    },
};
const INFURA_API_KEY: &str = "";

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
            EncodeEVMAddress => {
                name: "address",
                documentation: "`evm::address` creates a valid Ethereum address from the input string.",
                example: indoc! {r#"
                        output "address" { 
                            value = evm::address("0x627306090abaB3A6e1400e9345bC60c78a8BEf57")
                        }
                        "#},
                inputs: [
                    address_string: {
                        documentation: "An Ethereum address string.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The input string as an Ethereum address.",
                    typing: Type::addon(EVM_ADDRESS)
                },
            }
        },
        define_function! {
            EncodeEVMBytes32 => {
                name: "bytes32",
                documentation: "`evm::bytes32` encodes a hex string as a 32-byte buffer.",
                example: indoc! {r#"
                        output "32_bytes" {
                            value = evm::bytes32("0123456789012345678901234567890123456789012345678901234567890123")
                        }
                        "#},
                inputs: [
                    input: {
                        documentation: "A 32-byte hexadecimal string.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "A 32-byte buffer.",
                    typing: Type::addon(EVM_BYTES32)
                },
            }
        },
        define_function! {
            EncodeEVMBytes => {
                name: "bytes",
                documentation: "`evm::bytes` encodes a hex string as a variable length buffer.",
                example: indoc! {r#"
                        output "bytes" {
                            value = evm::bytes(encode_hex("Hello, world!"))
                        }
                        "#},
                inputs: [
                    input: {
                        documentation: "The hex string to encode.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The input string encoded as a buffer.",
                    typing: Type::addon(EVM_BYTES)
                },
            }
        },
        define_function! {
            EncodeEVMChain => {
                name: "chain",
                documentation: "`evm::chain` generates a default chain id and RPC API URL for a valid EVM compatible chain name.",
                example: indoc! {r#"
                        output "chain_id" {
                            value = evm::chain("optimism")
                        }
                        // > chain_id: 10
                        "#},
                inputs: [
                    input: {
                        documentation: "A EVM-compatible chain name. See https://chainlist.org for a list of supported chains.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "The default chain data.",
                    typing: CHAIN_DEFAULTS.clone()
                },
            }
        },
        define_function! {
            GetFoundryDeploymentArtifacts => {
                name: "get_contract_from_foundry_project",
                documentation: "Coming soon",
                example: indoc! {r#"
                        // Coming Soon
                        "#},
                inputs: [
                    input: {
                        documentation: "Coming Soon",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                },
            }
        },
        define_function! {
            GetHardhatDeploymentArtifacts => {
                name: "get_contract_from_hardhat_project",
                documentation: "Coming soon",
                example: indoc! {r#"
                variable "contract" {
                    value = evm::get_contract_from_hardhat_project(
                        "path/to/hardhat/artifacts",
                        "contracts/MyContract.sol",
                        "MyContract"
                    )
                }
                output "abi" {
                    value = variable.contract.abi
                } 
                "#},
                inputs: [
                    artifacts_path: {
                        documentation: "The path to the Hardhat artifacts directory.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_source_path: {
                        documentation: "The path, relative to the Hardhat project root, to the contract source file.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_name: {
                        documentation: "The name of the contract being deployed.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                },
            }
        },
        define_function! {
            CreateInitCode => {
                name: "create_init_code",
                documentation: "Coming soon",
                example: indoc! {r#"
                        // Coming Soon
                        "#},
                inputs: [
                    bytecode: {
                        documentation: "Coming Soon",
                        typing: vec![Type::string()]
                    },
                    constructor_args: {
                        documentation: "Coming Soon",
                        typing: vec![Type::array(Type::string())]
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                },
            }
        },
        define_function! {
            GenerateCreate2Address => {
                name: "create2",
                documentation: "Coming soon",
                example: indoc! {r#"
                        // Coming Soon
                        "#},
                inputs: [
                    salt: {
                        documentation: "Coming Soon",
                        typing: vec![Type::string()]
                    },
                    init_code: {
                        documentation: "Coming Soon",
                        typing: vec![Type::addon(EVM_INIT_CODE), Type::string()]
                    },
                    create2_factory_contract_address: {
                        documentation: "Coming Soon",
                        typing: vec![Type::addon(EVM_ADDRESS), Type::string()]
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                },
            }
        }
    ];
}

#[derive(Clone)]
pub struct EncodeEVMAddress;
impl FunctionImplementation for EncodeEVMAddress {
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
        let entry = match args.get(0) {
            Some(Value::String(val)) => val.clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::address' function: expected string, got {:?}",
                    other
                ))
            }
        };
        let address = string_to_address(entry)
            .map_err(|e| diagnosed_error!("'evm::address' function: {e}"))?;
        let bytes = address.0 .0.to_vec();
        Ok(EvmValue::address(bytes))
    }
}

#[derive(Clone)]
pub struct EncodeEVMBytes32;
impl FunctionImplementation for EncodeEVMBytes32 {
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
        let bytes = match args.get(0) {
            Some(Value::String(val)) => B256::from_hex(&val).map_err(|e| {
                diagnosed_error!("'evm::bytes32' function: failed to parse string: {:?}", e)
            })?,
            Some(Value::Addon(addon_value)) => {
                B256::try_from(&addon_value.bytes[..]).map_err(|e| {
                    diagnosed_error!("'evm::bytes32' function: failed to parse bytes: {}", e)
                })?
            }
            other => {
                return Err(diagnosed_error!(
                    "'evm::bytes32' function: expected string, got {:?}",
                    other
                ))
            }
        };
        Ok(EvmValue::bytes32(bytes.to_vec()))
    }
}

#[derive(Clone)]
pub struct EncodeEVMBytes;
impl FunctionImplementation for EncodeEVMBytes {
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
        let bytes = match args.get(0) {
            Some(Value::String(val)) => Bytes::from_hex(&val).map_err(|e| {
                diagnosed_error!("'evm::bytes function: failed to parse string: {:?}", e)
            })?,
            Some(Value::Addon(addon_value)) => Bytes::copy_from_slice(&addon_value.bytes[..]),
            other => {
                return Err(diagnosed_error!(
                    "'evm::bytes' function: expected string, got {:?}",
                    other
                ))
            }
        };
        Ok(EvmValue::bytes(bytes.to_vec()))
    }
}

#[derive(Clone)]
pub struct EncodeEVMChain;
impl FunctionImplementation for EncodeEVMChain {
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
        let chain = match args.get(0) {
            Some(Value::String(chain_name)) => {
                alloy_chains::Chain::from_str(&chain_name.to_ascii_lowercase()).map_err(|e| {
                    diagnosed_error!(
                        "'evm::chain' function: invalid chain name {}: {}",
                        chain_name,
                        e
                    )
                })?
            }
            Some(Value::Integer(chain_id)) => alloy_chains::Chain::from_id(*chain_id as u64),
            other => {
                return Err(diagnosed_error!(
                    "'evm::chain' function: expected string, got {:?}",
                    other
                ))
            }
        };

        let chain_alias = match chain.into_kind() {
            ChainKind::Named(name) => name.to_string(),
            ChainKind::Id(id) => id.to_string(),
        };

        let rpc_api_url = match args.get(1) {
            Some(Value::String(val)) => val.to_ascii_lowercase().clone(),
            None => format!("https://{}.infura.io/v3/{}", &chain_alias, INFURA_API_KEY),
            other => {
                return Err(diagnosed_error!(
                    "'evm::chain' function: expected string, got {:?}",
                    other
                ))
            }
        };

        let obj_props = IndexMap::from([
            ("chain_id".into(), Value::integer(chain.id() as i128)),
            ("rpc_api_url".into(), Value::string(rpc_api_url)),
            ("chain_alias".into(), Value::string(chain_alias)),
        ]);
        Ok(Value::object(obj_props))
    }
}

#[derive(Clone)]
pub struct GetFoundryDeploymentArtifacts;
impl FunctionImplementation for GetFoundryDeploymentArtifacts {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let prefix = "command 'evm::get_contract_from_foundry_project'";
        let manifest_file_path = match args.get(0) {
            Some(Value::String(manifest_path)) => {
                let path = Path::new(manifest_path);
                if path.is_absolute() {
                    FileLocation::from_path(path.to_path_buf())
                } else {
                    let mut workspace_loc =
                        auth_ctx.workspace_location.get_parent_location().map_err(|e| {
                            diagnosed_error!(
                                "{}: unable to read workspace location: {}",
                                prefix,
                                e.to_string()
                            )
                        })?;
                    workspace_loc.append_path(&manifest_path.to_string()).map_err(|e| {
                        diagnosed_error!("{}: invalid foundry manifest path: {}", prefix, e)
                    })?;
                    workspace_loc
                }
            }
            other => return Err(format_fn_error(&prefix, 1, "string", other)),
        };
        let contract_filename = match args.get(1) {
            Some(Value::String(val)) => val.clone(),
            other => return Err(format_fn_error(&prefix, 2, "string", other)),
        };

        let contract_name = match args.get(2) {
            Some(Value::String(val)) => val.clone(),
            None => contract_filename.clone(),
            other => return Err(format_fn_error(&prefix, 3, "string", other)),
        };

        let foundry_profile = match args.get(3) {
            Some(Value::String(val)) => val.clone(),
            None => "default".into(),
            other => return Err(format_fn_error(&prefix, 4, "string", other)),
        };

        let foundry_config = FoundryConfig::get_from_path(&manifest_file_path)
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;

        let compiled_output = foundry_config
            .get_compiled_output(&contract_filename, &contract_name, Some(&foundry_profile))
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;

        let abi_string = serde_json::to_string(&compiled_output.abi)
            .map_err(|e| diagnosed_error!("{}: failed to serialize abi: {}", prefix, e))?;

        let source = compiled_output
            .get_contract_source(&manifest_file_path, &contract_filename)
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;

        let abi = Value::string(abi_string);
        let bytecode = Value::string(compiled_output.bytecode.object.clone());
        let source = Value::string(source);
        let compiler_version =
            Value::string(format!("v{}", compiled_output.metadata.compiler.version));
        let contract_name = Value::string(contract_name.to_string());
        let optimizer_enabled = Value::bool(compiled_output.metadata.settings.optimizer.enabled);
        let optimizer_runs =
            Value::integer(compiled_output.metadata.settings.optimizer.runs as i128);
        let evm_version = Value::string(compiled_output.metadata.settings.evm_version);

        let mut obj_props = indexmap::indexmap! {
            "abi".to_string() => abi,
            "bytecode".to_string() => bytecode,
            "source".to_string() => source,
            "compiler_version".to_string() => compiler_version,
            "contract_name".to_string() => contract_name,
            "optimizer_enabled".to_string() => optimizer_enabled,
            "optimizer_runs".to_string() => optimizer_runs,
            "evm_version".to_string() => evm_version,
        };
        if let Some(via_ir) = compiled_output.metadata.settings.via_ir {
            let via_ir = Value::bool(via_ir);
            obj_props.insert("via_ir".to_string(), via_ir);
        }
        Ok(Value::object(obj_props))
    }
}

#[derive(Clone)]
pub struct GetHardhatDeploymentArtifacts;
impl FunctionImplementation for GetHardhatDeploymentArtifacts {
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
        let artifacts_path_str = args.get(0).unwrap().as_string().unwrap();
        let contract_source_path_str = args.get(1).unwrap().as_string().unwrap();
        let contract_name = args.get(2).unwrap().as_string().unwrap();

        let artifacts_path = Path::new(artifacts_path_str);
        let artifacts_path = if artifacts_path.is_absolute() {
            FileLocation::from_path(artifacts_path.to_path_buf())
        } else {
            let mut workspace_loc = auth_ctx
                .workspace_location
                .get_parent_location()
                .map_err(|e| to_diag(fn_spec, format!("unable to read workspace location: {e}")))?;

            workspace_loc
                .append_path(&artifacts_path_str.to_string())
                .map_err(|e| to_diag(fn_spec, format!("invalid hardhat config path: {}", e)))?;
            workspace_loc
        };

        let HardhatBuildArtifacts { compiled_contract_path, artifacts, build_info } =
            HardhatBuildArtifacts::new(
                artifacts_path.expect_path_buf(),
                &contract_source_path_str,
                &contract_name,
            )
            .map_err(|e| to_diag(fn_spec, e))?;

        let abi_string = serde_json::to_string(&artifacts.abi)
            .map_err(|e| to_diag(fn_spec, format!("failed to serialize abi: {}", e)))?;

        let source = build_info.input.sources.get(&artifacts.source_name).ok_or(to_diag(
            fn_spec,
            format!(
                "hardhat project output missing contract source for {}",
                contract_source_path_str
            ),
        ))?;

        let bytecode = Value::string(artifacts.bytecode);
        let abi = Value::string(abi_string);
        let source = Value::string(source.content.clone());
        let compiler_version = Value::string(build_info.solc_long_version);
        let contract_name = Value::string(contract_name.to_string());
        let contract_target_path = Value::string(artifacts.source_name);
        let optimizer_enabled = Value::bool(build_info.input.settings.optimizer.enabled);
        let optimizer_runs = Value::integer(build_info.input.settings.optimizer.runs as i128);
        let evm_version = Value::string(build_info.input.settings.evm_version);
        let project_root = Value::string(compiled_contract_path.to_string());
        // let foundry_config = Value::buffer(serde_json::to_vec(&config).unwrap());

        let obj_props = ObjectType::from(vec![
            ("bytecode", bytecode),
            ("abi", abi),
            ("source", source),
            ("compiler_version", compiler_version),
            ("contract_name", contract_name),
            ("contract_target_path", contract_target_path),
            ("optimizer_enabled", optimizer_enabled),
            ("optimizer_runs", optimizer_runs),
            ("evm_version", evm_version),
            ("project_root", project_root),
            // ("foundry_config", foundry_config),
        ]);

        Ok(Value::object(obj_props.inner()))
    }
}

#[derive(Clone)]
pub struct CreateInitCode;
impl FunctionImplementation for CreateInitCode {
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
        let prefix = "command 'evm::create_init_code'";
        let bytecode = match args.get(0) {
            Some(Value::String(val)) => val.clone(),
            other => return Err(format_fn_error(&prefix, 1, "string", other)),
        };
        let constructor_args = match args.get(1) {
            Some(constructor_args) => {
                let sol_args = constructor_args
                    .expect_array()
                    .iter()
                    .map(|v| value_to_sol_value(&v))
                    .collect::<Result<Vec<DynSolValue>, String>>()
                    .map_err(|e| {
                        diagnosed_error!(
                            "{}, argument position 2: failed to encode solidity value: {}",
                            prefix,
                            e
                        )
                    })?;
                sol_args
            }
            other => return Err(format_fn_error(&prefix, 2, "array", other)),
        };
        let init_code = create_init_code(bytecode, Some(constructor_args), None)
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;
        Ok(EvmValue::init_code(init_code))
    }
}

#[derive(Clone)]
pub struct GenerateCreate2Address;
impl FunctionImplementation for GenerateCreate2Address {
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
        let prefix = "command 'evm::create2'";
        let salt = match args.get(0) {
            Some(Value::String(salt)) => salt,
            other => return Err(format_fn_error(&prefix, 1, "string", other)),
        };

        let init_code = match args.get(1) {
            Some(Value::String(init_code)) => alloy::hex::decode(init_code)
                .map_err(|e| diagnosed_error!("{}: failed to decode init_code: {}", prefix, e))?,
            Some(Value::Addon(addon_data)) => {
                if addon_data.id != EVM_INIT_CODE {
                    return Err(format_fn_error(
                        &prefix,
                        2,
                        "string or ETH_INIT_CODE",
                        Some(&Value::Addon(addon_data.clone())),
                    ));
                }
                addon_data.bytes.clone()
            }
            other => return Err(format_fn_error(&prefix, 2, "string", other)),
        };

        let factory_address = args
            .get(2)
            .and_then(|v| Some(v.clone()))
            .unwrap_or(Value::string(DEFAULT_CREATE2_FACTORY_ADDRESS.to_string()));

        let create2 = generate_create2_address(&factory_address, &salt, &init_code)
            .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;
        let bytes = create2.0 .0.to_vec();
        Ok(EvmValue::address(bytes))
    }
}

fn format_fn_error(ctx: &str, position: u64, expected: &str, actual: Option<&Value>) -> Diagnostic {
    return diagnosed_error!(
        "'{}', argument position {:?}: expected {}, got {:?}",
        ctx,
        position,
        expected,
        actual.and_then(|v| Some(v.get_type()))
    );
}
