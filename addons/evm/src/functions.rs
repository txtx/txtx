use std::{path::Path, str::FromStr};

use crate::typing::LINKED_LIBRARIES_TYPE;
use alloy::{
    dyn_abi::DynSolValue,
    hex::FromHex,
    json_abi::JsonAbi,
    primitives::{Address, Bytes, B256},
};
use alloy_chains::ChainKind;
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    types::functions::{arg_checker_with_ctx, fn_diag_with_ctx},
};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        namespace::Namespace,
        types::{ObjectType, Type, Value},
        AuthorizationContext,
    },
};

use crate::{
    codec::{
        contract_deployment::{create_init_code, create_opts::generate_create2_address},
        foundry::FoundryToml,
        hardhat::HardhatBuildArtifacts,
        string_to_address, value_to_abi_function_args, value_to_sol_value,
    },
    commands::actions::call_contract::{
        encode_contract_call_inputs_from_abi_str, encode_contract_call_inputs_from_selector,
    },
    constants::{
        DEFAULT_CREATE2_FACTORY_ADDRESS, DEFAULT_FOUNDRY_MANIFEST_PATH, DEFAULT_FOUNDRY_PROFILE,
        DEFAULT_HARDHAT_ARTIFACTS_DIR, DEFAULT_HARDHAT_SOURCE_DIR, NAMESPACE,
    },
    typing::{
        decode_hex, EvmValue, CHAIN_DEFAULTS, DEPLOYMENT_ARTIFACTS_TYPE, EVM_ADDRESS, EVM_BYTES,
        EVM_BYTES32, EVM_FOUNDRY_BYTECODE_DATA, EVM_FUNCTION_CALL, EVM_INIT_CODE, EVM_UINT256,
        EVM_UINT32, EVM_UINT8,
    },
};
const INFURA_API_KEY: &str = "";

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &[Value]) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(Namespace::from(NAMESPACE));
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(Namespace::from(NAMESPACE));
    error_fn(fn_spec, e)
}

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeEvmAddress => {
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
            EvmZeroAddress => {
                name: "zero_address",
                documentation: "`evm::zero_address` is a constant representing the zero address.",
                example: indoc! {r#"
                        output "address" { 
                            value = evm::zero_address()
                        }
                        "#},
                inputs: [],
                output: {
                    documentation: "The zero address, `0x0000000000000000000000000000000000000000`.",
                    typing: Type::addon(EVM_ADDRESS)
                },
            }
        },
        define_function! {
            EncodeToAbiType => {
                name: "to_abi_type",
                documentation: "`evm::to_abi_type` is coming soon",
                example: indoc! {r#"
                        
                "#},
                inputs: [
                    value: {
                        documentation: "Coming soon.",
                        typing: vec![Type::string(), Type::addon(""), Type::array(Type::string())]
                    },
                    abi: {
                        documentation: "Coming soon.",
                        typing: vec![Type::string()]
                    },
                    typing: {
                        documentation: "Coming soon.",
                        typing: vec![Type::string()]
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: Type::addon(EVM_BYTES32)
                },
            }
        },
        define_function! {
            AbiEncode => {
                name: "abi_encode",
                documentation: "`evm::abi_encode` is coming soon",
                example: indoc! {r#"
                        
                "#},
                inputs: [
                    input: {
                        documentation: "Coming soon.",
                        typing: vec![Type::array(Type::string())]
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: Type::addon(EVM_BYTES32)
                },
            }
        },
        define_function! {
            EncodeEvmBytes32 => {
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
            EncodeEvmBytes => {
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
                        typing: vec![Type::string(), Type::array(Type::string()), Type::addon(""), Type::buffer()]
                    }
                ],
                output: {
                    documentation: "The input string encoded as a buffer.",
                    typing: Type::addon(EVM_BYTES)
                },
            }
        },
        define_function! {
            EncodeEvmUint256 => {
                name: "uint256",
                documentation: "`evm::uint256` encodes a number as a Solidity uint256 value.",
                example: indoc! {r#"
                        output "uint256" {
                            value = evm::uint256(1)
                        }
                        "#},
                inputs: [
                    input: {
                        documentation: "The number to encode.",
                        typing: vec![Type::string(), Type::array(Type::string()), Type::addon(""), Type::buffer()]
                    }
                ],
                output: {
                    documentation: "The number encoded as a Solidity uint256.",
                    typing: Type::addon(EVM_UINT256)
                },
            }
        },
        define_function! {
            EncodeEvmUint32 => {
                name: "uint32",
                documentation: "`evm::uint32` encodes a number as a Solidity uint32 value.",
                example: indoc! {r#"
                        output "uint32" {
                            value = evm::uint32(1)
                        }
                        "#},
                inputs: [
                    input: {
                        documentation: "The number to encode.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The number encoded as a Solidity uint32.",
                    typing: Type::addon(EVM_UINT32)
                },
            }
        },
        define_function! {
            EncodeEvmUint8 => {
                name: "uint8",
                documentation: "`evm::uint8` encodes a number as a Solidity uint8 value.",
                example: indoc! {r#"
                        output "uint8" {
                            value = evm::uint8(1)
                        }
                        "#},
                inputs: [
                    input: {
                        documentation: "The number to encode.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The number encoded as a Solidity uint8.",
                    typing: Type::addon(EVM_UINT8)
                },
            }
        },
        define_function! {
            EncodeEvmChain => {
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
                        documentation: "An EVM-compatible chain name. See https://chainlist.org for a list of supported chains.",
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
                documentation: "`evm::get_contract_from_foundry_project` retrieves the compiled contract artifacts for a contract in a Foundry project.",
                example: indoc! {r#"
                variable "contract" {
                    value = evm::get_contract_from_foundry_project("MyContract")
                }
                output "abi" {
                    value = variable.contract.abi
                }        
                "#},
                inputs: [
                    contract_name: {
                        documentation: "The name of the contract being deployed.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_filename: {
                        documentation: "The `.sol` file that the contract is located in. Defaults to `<ContractName>.sol`.",
                        typing: vec![Type::string()],
                        optional: true
                    },
                    foundry_manifest_path: {
                        documentation: "The location of the Foundry.toml. Defaults to `./foundry.toml`.",
                        typing: vec![Type::string()],
                        optional: true
                    },
                    foundry_profile: {
                        documentation: "The foundry profile that should be used to find the compiled output. Defaults to `default`.",
                        typing: vec![Type::string()],
                        optional: true
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
                documentation: "`evm::get_contract_from_hardhat_project` retrieves the compiled contract artifacts for a contract in a Hardhat project.",
                example: indoc! {r#"
                variable "contract" {
                    value = evm::get_contract_from_hardhat_project("MyContract")
                }
                output "abi" {
                    value = variable.contract.abi
                } 
                "#},
                inputs: [
                    contract_name: {
                        documentation: "The name of the contract being deployed.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_source_path: {
                        documentation: "The path, relative to the Hardhat project root, to the contract source file. Defaults to `./contracts/<ContractName>.sol`.",
                        typing: vec![Type::string()],
                        optional: true
                    },
                    artifacts_path: {
                        documentation: "The path to the Hardhat artifacts directory. Defaults to `./artifacts`.",
                        typing: vec![Type::string()],
                        optional: true
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
                        typing: vec![Type::string(), Type::addon(EVM_FOUNDRY_BYTECODE_DATA)]
                    },
                    constructor_args: {
                        documentation: "Coming Soon",
                        typing: vec![Type::array(Type::string())]
                    },
                    linked_libraries: {
                        documentation: "Coming Soon",
                        typing: vec![LINKED_LIBRARIES_TYPE.clone()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: Type::addon(EVM_INIT_CODE)
                },
            }
        },
        define_function! {
            EncodeFunctionCall => {
                name: "encode_function_call",
                documentation: "Coming soon",
                example: indoc! {r#"
                        // Coming Soon
                        "#},
                inputs: [
                    function_name: {
                        documentation: "Coming Soon",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    function_args: {
                        documentation: "Coming Soon",
                        typing: vec![Type::array(Type::string()), Type::array(Type::integer()), Type::array(Type::addon(EVM_ADDRESS))],
                        optional: false
                    },
                    abi: {
                        documentation: "Coming Soon",
                        typing: vec![Type::string()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "Coming Soon",
                    typing: Type::addon(EVM_FUNCTION_CALL)
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
pub struct EncodeEvmAddress;
impl FunctionImplementation for EncodeEvmAddress {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
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
        Ok(EvmValue::address(&address))
    }
}

#[derive(Clone)]
pub struct EvmZeroAddress;
impl FunctionImplementation for EvmZeroAddress {
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
        Ok(EvmValue::address(&Address::ZERO))
    }
}

#[derive(Clone)]
pub struct AbiEncode;
impl FunctionImplementation for AbiEncode {
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
        let array = args.get(0).unwrap().as_array().unwrap();

        let sol_values = array
            .iter()
            .enumerate()
            .map(|(i, val)| {
                value_to_sol_value(val)
                    .map_err(|e| diagnosed_error!("failed to encode value #{}: {}", i + 1, e))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;

        // the solidity abi.encode function doesn't just concatenate each entry's encoded bytes.
        // it has dynamic data encoding rules. so, we need to wrap the values in a Tuple type,
        // then encode as a sequence (which removes the Tuple wrapper)
        Ok(EvmValue::bytes(DynSolValue::Tuple(sol_values).abi_encode_sequence().unwrap()))
    }
}

#[derive(Clone)]
pub struct EncodeToAbiType;
impl FunctionImplementation for EncodeToAbiType {
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
        let value = args.get(0).unwrap();
        let abi = args.get(1).unwrap().as_string().unwrap();
        let type_name = args.get(2).unwrap().as_string().unwrap();
        let abi: JsonAbi = serde_json::from_str(abi)
            .map_err(|e| diagnosed_error!("failed to parse abi: {}", e))?;

        // look through all of the abi functions to find an internal type
        // (either in the function inputs or outputs) that matches our type_name
        let fn_param_matches = abi
            .functions
            .values()
            .flat_map(|fs| {
                fs.iter()
                    .filter_map(|f| {
                        if let Some(found_input_ty) = f.inputs.iter().find(|i| {
                            i.internal_type
                                .as_ref()
                                .map(|i| i.to_string() == type_name)
                                .unwrap_or(false)
                        }) {
                            return Some(found_input_ty);
                        }

                        if let Some(found_output_ty) = f.outputs.iter().find(|i| {
                            i.internal_type
                                .as_ref()
                                .map(|i| i.to_string() == type_name)
                                .unwrap_or(false)
                        }) {
                            return Some(found_output_ty);
                        }
                        None
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        if !fn_param_matches.is_empty() {
            let param = fn_param_matches.get(0).unwrap();
            return Ok(EvmValue::known_sol_param(value, param));
        }
        Err(diagnosed_error!("no type found in abi matching type name: {}", type_name))
    }
}

#[derive(Clone)]
pub struct EncodeEvmBytes32;
impl FunctionImplementation for EncodeEvmBytes32 {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
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
pub struct EncodeEvmBytes;
impl FunctionImplementation for EncodeEvmBytes {
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
        let bytes = match args.get(0).unwrap() {
            Value::String(val) => Bytes::from_hex(&val).map_err(|e| {
                diagnosed_error!("'evm::bytes function: failed to parse string: {:?}", e)
            })?,
            Value::Addon(addon_value) => Bytes::copy_from_slice(&addon_value.bytes[..]),
            other => {
                let bytes = other.to_be_bytes();
                Bytes::copy_from_slice(&bytes)
            }
        };
        Ok(EvmValue::bytes(bytes.to_vec()))
    }
}

#[derive(Clone)]
pub struct EncodeEvmUint32;
impl FunctionImplementation for EncodeEvmUint32 {
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

        Ok(EvmValue::uint32(value.to_be_bytes().as_slice().to_vec()))
    }
}

#[derive(Clone)]
pub struct EncodeEvmUint256;
impl FunctionImplementation for EncodeEvmUint256 {
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
        let value = args.get(0).unwrap();
        match value {
            Value::String(s) => {
                let normalized = if s.starts_with("0x") {
                    decode_hex(&s)?
                } else {
                    let u = alloy::primitives::U256::from_str_radix(&s, 10)
                        .map_err(|e| diagnosed_error!("failed to parse string as number: {}", e))?;
                    u.to_be_bytes_vec()
                };
                Ok(EvmValue::uint256(normalized))
            }
            _ => Ok(EvmValue::uint256(value.to_be_bytes())),
        }
    }
}

#[derive(Clone)]
pub struct EncodeEvmUint8;
impl FunctionImplementation for EncodeEvmUint8 {
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

        Ok(EvmValue::uint8(value.to_be_bytes().as_slice().to_vec()))
    }
}

#[derive(Clone)]
pub struct EncodeEvmChain;
impl FunctionImplementation for EncodeEvmChain {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
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
        let contract_name = args.get(0).unwrap().as_string().unwrap();
        let default_contract_filename = format!("{}.sol", contract_name);
        let contract_filename =
            args.get(1).and_then(|v| v.as_string()).unwrap_or(&default_contract_filename);
        let manifest_path_str =
            args.get(2).and_then(|v| v.as_string()).unwrap_or(DEFAULT_FOUNDRY_MANIFEST_PATH);
        let foundry_profile =
            args.get(3).and_then(|v| v.as_string()).unwrap_or(DEFAULT_FOUNDRY_PROFILE);

        let manifest_path = Path::new(manifest_path_str);
        let manifest_path = if manifest_path.is_absolute() {
            FileLocation::from_path(manifest_path.to_path_buf())
        } else {
            let mut workspace_loc = auth_ctx
                .workspace_location
                .get_parent_location()
                .map_err(|e| to_diag(fn_spec, format!("unable to read workspace location: {e}")))?;

            workspace_loc
                .append_path(&manifest_path_str.to_string())
                .map_err(|e| to_diag(fn_spec, format!("invalid foundry manifest path: {}", e)))?;
            workspace_loc
        };

        let foundry_toml = FoundryToml::new(&manifest_path).map_err(|e| to_diag(fn_spec, e))?;

        let foundry_config = foundry_toml
            .get_foundry_config(Some(&foundry_profile))
            .map_err(|e| to_diag(fn_spec, format!("failed to get foundry config: {}", e)))?;

        let compiled_output = foundry_toml
            .get_compiled_output(&contract_name, &contract_filename, Some(&foundry_profile))
            .map_err(|e| to_diag(fn_spec, e))?;

        let abi_string = serde_json::to_string(&compiled_output.abi)
            .map_err(|e| to_diag(fn_spec, format!("failed to serialize abi: {}", e)))?;

        let source = compiled_output
            .get_contract_source(&manifest_path, &contract_name)
            .map_err(|e| to_diag(fn_spec, e))?;

        let target_path = compiled_output
            .get_contract_path(&manifest_path, &contract_name)
            .map_err(|e| to_diag(fn_spec, e))
            .map(|path| FileLocation::from_path(path))?
            .get_absolute_path()
            .map_err(|e| {
                to_diag(fn_spec, format!("could not find compilation target path: {e}"))
            })?;
        let target_path = target_path.to_str().ok_or_else(|| {
            to_diag(
                fn_spec,
                format!("invalid compilation target path for contract {}", contract_name),
            )
        })?;
        let bytecode = EvmValue::foundry_bytecode_data(&compiled_output.bytecode)
            .map_err(|e| to_diag(fn_spec, e.message))?;
        let abi = Value::string(abi_string);
        let source = Value::string(source);
        let contract_name = Value::string(contract_name.to_string());
        let contract_target_path = Value::string(target_path.to_string());
        let deployed_bytecode = EvmValue::foundry_bytecode_data(&compiled_output.deployed_bytecode)
            .map_err(|e| to_diag(fn_spec, e.message))?;

        let metadata = EvmValue::foundry_compiled_metadata(&compiled_output.metadata)
            .map_err(|e| to_diag(fn_spec, e.message))?;

        let foundry_config =
            Value::buffer(serde_json::to_vec(&foundry_config).map_err(|e| {
                to_diag(fn_spec, format!("failed to serialize foundry config: {}", e))
            })?);

        let obj_props = ObjectType::from([
            ("bytecode", bytecode),
            ("deployed_bytecode", deployed_bytecode),
            ("abi", abi),
            ("source", source),
            ("contract_name", contract_name),
            ("contract_filename", Value::string(contract_filename.to_string())),
            ("contract_target_path", contract_target_path),
            ("foundry_config", foundry_config),
            ("metadata", metadata),
        ]);

        Ok(Value::object(obj_props.inner()))
    }
}

#[derive(Clone)]
pub struct GetHardhatDeploymentArtifacts;
impl FunctionImplementation for GetHardhatDeploymentArtifacts {
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
        let contract_name = args.get(0).unwrap().as_string().unwrap();
        let default_source_path = format!("{}/{}.sol", DEFAULT_HARDHAT_SOURCE_DIR, contract_name);
        let contract_source_path_str =
            args.get(1).and_then(|v| v.as_string()).unwrap_or(&default_source_path);
        let artifacts_path_str =
            args.get(2).and_then(|v| v.as_string()).unwrap_or(DEFAULT_HARDHAT_ARTIFACTS_DIR);

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

        let source = build_info.input.sources.get(&artifacts.source_name).ok_or_else(|| {
            to_diag(
                fn_spec,
                format!(
                    "hardhat project output missing contract source for {}",
                    contract_source_path_str
                ),
            )
        })?;

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
        let prefix = "command 'evm::create_init_code'";
        let bytecode = match args.get(0) {
            Some(Value::String(val)) => {
                crate::codec::foundry::BytecodeData { object: val.clone(), ..Default::default() }
            }
            Some(Value::Addon(addon_data)) => {
                EvmValue::to_foundry_bytecode_data(&Value::Addon(addon_data.clone()))
                    .map_err(|d| to_diag(fn_spec, d.to_string()))?
            }
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

        let linked_libraries = args
            .get(2)
            .map(|lib| {
                let lib = lib.as_object().unwrap();
                lib.iter()
                    .map(|(k, v)| EvmValue::to_address(v).map(|a| (k.clone(), a)))
                    .collect::<Result<IndexMap<String, Address>, _>>()
            })
            .transpose()
            .map_err(|d| {
                to_diag(fn_spec, format!("each entry of a linked library must be an address: {d}"))
            })?;
        let init_code = create_init_code(bytecode, Some(constructor_args), &None, linked_libraries)
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;
        Ok(EvmValue::init_code(init_code))
    }
}

#[derive(Clone)]
pub struct EncodeFunctionCall;
impl FunctionImplementation for EncodeFunctionCall {
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
        let function_name = args.get(0).unwrap().as_string().unwrap();
        let function_args = args.get(1).unwrap();

        let abi = args
            .get(2)
            .map(|abi| {
                abi.as_string().ok_or_else(|| {
                    to_diag(fn_spec, format!("argument #3 (abi) should be of type (string)"))
                })
            })
            .transpose()?;

        let input = if let Some(abi_str) = abi {
            let abi: JsonAbi = serde_json::from_str(&abi_str)
                .map_err(|e| to_diag(fn_spec, format!("invalid contract abi: {}", e)))?;

            let function_args = value_to_abi_function_args(&function_name, &function_args, &abi)
                .map_err(|e| to_diag(fn_spec, e.message))?;

            encode_contract_call_inputs_from_abi_str(abi_str, &function_name, &function_args)
                .map_err(|e| to_diag(fn_spec, e))?
        } else {
            let function_args = function_args
                .as_array()
                .unwrap()
                .iter()
                .map(|v| value_to_sol_value(&v).map_err(|e| to_diag(fn_spec, e)))
                .collect::<Result<Vec<DynSolValue>, Diagnostic>>()?;

            encode_contract_call_inputs_from_selector(&function_name, &function_args)
                .map_err(|e| to_diag(fn_spec, e))?
        };
        Ok(EvmValue::function_call(input))
    }
}

#[derive(Clone)]
pub struct GenerateCreate2Address;
impl FunctionImplementation for GenerateCreate2Address {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
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
        Ok(EvmValue::address(&create2))
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
