use alloy::dyn_abi::DynSolValue;
use alloy::hex;
use alloy::json_abi::JsonAbi;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::codec::foundry::FoundryConfig;
use crate::codec::value_to_sol_value;
use crate::constants::CONTRACT_CONSTRUCTOR_ARGS;
use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

lazy_static! {
    pub static ref GET_FORGE_DEPLOYMENT_ARTIFACTS: PreCommandSpecification = {
        let mut command = define_command! {
            GetForgeDeploymentArtifacts => {
                name: "Get Forge Deployment Artifacts",
                matcher: "get_forge_deployment_artifacts",
                documentation: "The `evm::get_forge_deployment_artifacts` command gets all artifacts from a forge project that are required for deploying and verifying a contract.",
                implements_signing_capability: false,
                implements_background_task_capability: false,
                inputs: [
                    description: {
                        documentation: "A description of the call.",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    foundry_toml_path: {
                        documentation: "The path to the Foundry project's `foundry.toml` file.",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    foundry_profile: {
                        documentation: "The profile to use from the `foundry.toml` file. The default is `default`.",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    contract_filename: {
                        documentation: "The name of the contract file contains the contract being deployed. For example, `SimpleStorage.sol`",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    contract_name: {
                        documentation: "The name of the contract being deployed. If omitted, the `contract_filename` argument (without extension) will be used.",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    constructor_args: {
                        documentation: "The constructor args to initialize a contract requiring constructor arguments..",
                        typing: Type::array(Type::buffer()),
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                        abi: {
                            documentation: "The contract abi.",
                            typing: Type::string()
                        },
                        bytecode: {
                            documentation: "The compiled contract bytecode.",
                            typing: Type::string()
                        },
                        constructor_args: {
                            documentation: "The abi encoded constructor arguments, if provided.",
                            typing: Type::string()
                        },
                        init_code: {
                            documentation: "The compiled contract bytecode concatenated with the abi encoded constructor arguments, if provided.",
                            typing: Type::string()
                        },
                        source: {
                            documentation: "The contract source code.",
                            typing: Type::string()
                        },
                        compiler_version: {
                            documentation: "The solc version used to compile the contract.",
                            typing: Type::string()
                        },
                        contract_name: {
                            documentation: "The name of the contract being deployed.",
                            typing: Type::string()
                        },
                        optimizer_enabled: {
                            documentation: "Whether the optimizer is enabled during contract compilation.",
                            typing: Type::bool()
                        },
                        optimizer_runs: {
                            documentation: "The number of runs the optimizer performed.",
                            typing: Type::uint()
                        },
                        evm_version: {
                            documentation: "The EVM version used to compile the contract.",
                            typing: Type::uint()
                        },
                        value: {
                            documentation: "The other outputs as one object.",
                            typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                        }
                ],
                example: txtx_addon_kit::indoc! {r#"
                // Coming soon
            "#},
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("source".to_string());
        }
        command
    };
}

pub struct GetForgeDeploymentArtifacts;
impl CommandImplementation for GetForgeDeploymentArtifacts {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let args = args.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let foundry_toml_path = args.get_expected_string("foundry_toml_path")?;
            let foundry_profile = args.get_string("foundry_profile");
            let contract_filename = args.get_expected_string("contract_filename")?;
            let contract_name = args
                .get_string("contract_name")
                .unwrap_or(contract_filename);
            let constructor_args =
                if let Some(function_args) = args.get_value(CONTRACT_CONSTRUCTOR_ARGS) {
                    let sol_args = function_args
                        .expect_array()
                        .iter()
                        .map(|v| {
                            value_to_sol_value(&v).map_err(|e| {
                                diagnosed_error!("command 'evm::sign_contract_call': {}", e)
                            })
                        })
                        .collect::<Result<Vec<DynSolValue>, Diagnostic>>()?;
                    Some(sol_args)
                } else {
                    None
                };

            let foundry_config = FoundryConfig::get_from_path(&foundry_toml_path)
                .await
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let compiled_output = foundry_config
                .get_compiled_output(contract_filename, contract_name, foundry_profile)
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let abi_string = serde_json::to_string(&compiled_output.abi).map_err(|e| {
                diagnosed_error!(
                    "'evm::get_forge_deployment_artifacts' function: failed to serialize abi: {e}"
                )
            })?;

            let mut init_code = compiled_output.bytecode.object.clone();
            let json_abi: JsonAbi = serde_json::from_str(&abi_string).map_err(|e| {
                diagnosed_error!(
                    "command 'get_forge_deployment_artifacts': failed to decode contract abi: {e}"
                )
            })?;

            let constructor_args = if let Some(constructor_args) = constructor_args {
                if json_abi.constructor.is_none() {
                    return Err(diagnosed_error!("command 'get_forge_deployment_artifacts': invalid arguments: constructor arguments provided, but abi has no constructor"));
                }
                let mut abi_encoded_args = constructor_args
                    .iter()
                    .flat_map(|s| s.abi_encode())
                    .collect::<Vec<u8>>();
                let mut hex_init_code = hex::decode(&init_code).map_err(|e| diagnosed_error!("command 'get_forge_deployment_artifacts': failed to decode contract bytecode: {e}"))?;
                let encoded_args = hex::encode(&abi_encoded_args);
                hex_init_code.append(&mut abi_encoded_args);
                init_code = hex::encode(hex_init_code);
                Some(Value::string(encoded_args))
            } else {
                if json_abi.constructor.is_some() {
                    return Err(diagnosed_error!("command 'get_forge_deployment_artifacts': invalid arguments: no constructor arguments provided, but abi has constructor"));
                }
                None
            };

            let source = compiled_output
                .get_contract_source(foundry_toml_path, contract_filename)
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let abi = Value::string(abi_string);
            let bytecode = Value::string(compiled_output.bytecode.object.clone());
            let init_code = Value::string(init_code);
            let source = Value::string(source);
            let compiler_version =
                Value::string(format!("v{}", compiled_output.metadata.compiler.version));
            let contract_name = Value::string(contract_name.to_string());
            let optimizer_enabled =
                Value::bool(compiled_output.metadata.settings.optimizer.enabled);
            let optimizer_runs = Value::uint(compiled_output.metadata.settings.optimizer.runs);
            let evm_version = Value::string(compiled_output.metadata.settings.evm_version);

            result.outputs.insert("abi".into(), abi.clone());
            result.outputs.insert("bytecode".into(), bytecode.clone());
            result.outputs.insert("init_code".into(), init_code.clone());
            result.outputs.insert("source".into(), source.clone());
            result
                .outputs
                .insert("compiler_version".into(), compiler_version.clone());
            result
                .outputs
                .insert("contract_name".into(), contract_name.clone());
            result
                .outputs
                .insert("optimizer_enabled".into(), optimizer_enabled.clone());
            result
                .outputs
                .insert("optimizer_runs".into(), optimizer_runs.clone());
            result
                .outputs
                .insert("evm_version".into(), evm_version.clone());

            let mut obj_props = HashMap::from([
                ("abi".to_string(), Ok(abi)),
                ("bytecode".to_string(), Ok(bytecode)),
                ("init_code".to_string(), Ok(init_code)),
                ("source".to_string(), Ok(source)),
                ("compiler_version".to_string(), Ok(compiler_version)),
                ("contract_name".to_string(), Ok(contract_name)),
                ("optimizer_enabled".to_string(), Ok(optimizer_enabled)),
                ("optimizer_runs".to_string(), Ok(optimizer_runs)),
                ("evm_version".to_string(), Ok(evm_version)),
            ]);
            if let Some(constructor_args) = constructor_args.clone() {
                result
                    .outputs
                    .insert("constructor_args".into(), constructor_args.clone());
                obj_props.insert("constructor_args".into(), Ok(constructor_args));
            }

            result
                .outputs
                .insert("value".into(), Value::object(obj_props));
            Ok(result)
        };

        Ok(Box::pin(future))
    }
}
