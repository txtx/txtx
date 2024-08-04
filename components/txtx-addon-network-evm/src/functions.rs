use std::str::FromStr;

use alloy::{
    hex::FromHex,
    primitives::{Address, Bytes, B256},
};
use txtx_addon_kit::indexmap;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{PrimitiveValue, Type, Value},
        AuthorizationContext,
    },
};

use crate::{
    codec::foundry::FoundryConfig,
    typing::{BYTES, BYTES32, CHAIN_DEFAULTS, DEPLOYMENT_ARTIFACTS_TYPE, ETH_ADDRESS},
};
const INFURA_API_KEY: &str = dotenv!("INFURA_API_KEY");

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
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The input string as an Ethereum address.",
                    typing: Type::uint()
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
                    typing: Type::addon(BYTES32.clone())
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
                    typing: Type::addon(BYTES.clone())
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
        let mut entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::address' function: expected string, got {:?}",
                    other
                ))
            }
        };
        entry = entry.replace("0x", "");
        // hack: we're assuming that if the address is 32 bytes, it's a sol value that's padded with 0s, so we trim them
        if entry.len() == 64 {
            let split_pos = entry.char_indices().nth_back(39).unwrap().0;
            entry = (entry[split_pos..]).to_owned();
        }
        let address = Address::from_hex(&entry)
            .map_err(|e| diagnosed_error!("'evm::address' function: invalid address: {}", e))?;
        let bytes = address.0 .0;
        Ok(Value::buffer(bytes.to_vec(), ETH_ADDRESS.clone()))
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
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::bytes32' function: expected string, got {:?}",
                    other
                ))
            }
        };
        let bytes = B256::from_hex(&entry).map_err(|e| {
            diagnosed_error!("'evm::bytes32' function: failed to parse string: {:?}", e)
        })?;
        let val = Value::buffer(bytes.to_vec(), BYTES32.clone());
        Ok(Value::addon(val, BYTES32.clone()))
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
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::bytes' function: expected string, got {:?}",
                    other
                ))
            }
        };
        let bytes = Bytes::from_hex(&entry).map_err(|e| {
            diagnosed_error!("'evm::bytes function: failed to parse string: {:?}", e)
        })?;
        let val = Value::buffer(bytes.to_vec(), BYTES32.clone());
        Ok(Value::addon(val, BYTES.clone()))
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
        let chain_name = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.to_ascii_lowercase().clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::chain' function: expected string, got {:?}",
                    other
                ))
            }
        };
        let chain = alloy_chains::Chain::from_str(&chain_name).map_err(|e| {
            diagnosed_error!(
                "'evm::chain' function: invalid chain name {}: {}",
                chain_name,
                e
            )
        })?;

        let rpc_api_url = match args.get(1) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.to_ascii_lowercase().clone(),
            None => format!("https://{}.infura.io/v3/{}", &chain_name, INFURA_API_KEY),
            other => {
                return Err(diagnosed_error!(
                    "'evm::chain' function: expected string, got {:?}",
                    other
                ))
            }
        };

        let obj_props = IndexMap::from([
            ("chain_id".into(), Value::uint(chain.id())),
            ("rpc_api_url".into(), Value::string(rpc_api_url)),
        ]);
        println!(
            "//// function result: {:?}",
            Value::object(obj_props.clone())
        );
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
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let prefix = "command 'evm::get_contract_from_foundry_project'";
        let manifest_file_path = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => return Err(format_fn_error(&prefix, 1, "string", other)),
        };
        let contract_filename = match args.get(1) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => return Err(format_fn_error(&prefix, 2, "string", other)),
        };

        let contract_name = match args.get(2) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            None => contract_filename.clone(),
            other => return Err(format_fn_error(&prefix, 3, "string", other)),
        };

        let foundry_profile = match args.get(3) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
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

        let init_code = compiled_output.bytecode.object.clone();

        let source = compiled_output
            .get_contract_source(&manifest_file_path, &contract_filename)
            .map_err(|e| diagnosed_error!("{}: {}", prefix, e))?;

        let abi = Value::string(abi_string);
        let bytecode = Value::string(compiled_output.bytecode.object.clone());
        let init_code = Value::string(init_code);
        let source = Value::string(source);
        let compiler_version =
            Value::string(format!("v{}", compiled_output.metadata.compiler.version));
        let contract_name = Value::string(contract_name.to_string());
        let optimizer_enabled = Value::bool(compiled_output.metadata.settings.optimizer.enabled);
        let optimizer_runs = Value::uint(compiled_output.metadata.settings.optimizer.runs);
        let evm_version = Value::string(compiled_output.metadata.settings.evm_version);

        let obj_props = indexmap::indexmap! {
            "abi".to_string() => abi,
            "bytecode".to_string() => bytecode,
            "init_code".to_string() => init_code,
            "source".to_string() => source,
            "compiler_version".to_string() => compiler_version,
            "contract_name".to_string() => contract_name,
            "optimizer_enabled".to_string() => optimizer_enabled,
            "optimizer_runs".to_string() => optimizer_runs,
            "evm_version".to_string() => evm_version,
        };
        Ok(Value::object(obj_props))
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
