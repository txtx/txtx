use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionFutureResult, CommandExecutionResult,
    CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::codec::foundry::FoundryConfig;
use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

lazy_static! {
    pub static ref GET_FORGE_DEPLOYMENT_ARTIFACTS: PreCommandSpecification = define_command! {
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let args = args.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let foundry_toml_path = args.get_expected_string("foundry_toml_path")?;
            let contract_filename = args.get_expected_string("contract_filename")?;
            let contract_name = args
                .get_string("contract_name")
                .unwrap_or(contract_filename);

            let foundry_config = FoundryConfig::get_from_path(&foundry_toml_path)
                .await
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let compiled_output = foundry_config
                .get_compiled_output(contract_filename, contract_name)
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let abi_string = serde_json::to_string(&compiled_output.abi).map_err(|e| {
                diagnosed_error!(
                    "'evm::get_forge_deployment_artifacts' function: failed to serialize abi: {e}"
                )
            })?;

            let source = compiled_output
                .get_contract_source(foundry_toml_path, contract_filename)
                .map_err(|e| {
                    diagnosed_error!("'evm::get_forge_deployment_artifacts' function: {e}")
                })?;

            let abi = Value::string(abi_string);
            let bytecode = Value::string(compiled_output.bytecode.object.clone());
            let source = Value::string(source);
            let compiler_version = Value::string(compiled_output.metadata.compiler.version);
            let contract_name = Value::string(contract_name.to_string());

            result.outputs.insert("abi".into(), abi.clone());
            result.outputs.insert("bytecode".into(), bytecode.clone());
            result.outputs.insert("source".into(), source.clone());
            result
                .outputs
                .insert("compiler_version".into(), compiler_version.clone());
            result
                .outputs
                .insert("contract_name".into(), contract_name.clone());

            result.outputs.insert(
                "value".into(),
                Value::object(HashMap::from([
                    ("abi".to_string(), Ok(abi)),
                    ("bytecode".to_string(), Ok(bytecode)),
                    ("source".to_string(), Ok(source)),
                    ("compiler_version".to_string(), Ok(compiler_version)),
                    ("contract_name".to_string(), Ok(contract_name)),
                ])),
            );
            println!("inserted forge artifacts value");

            Ok(result)
        };

        Ok(Box::pin(future))
    }
}
