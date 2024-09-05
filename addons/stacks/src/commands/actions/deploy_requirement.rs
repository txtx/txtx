use clarity::types::StacksEpochId;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::ClarityVersion;
use clarity_repl::analysis::ast_dependency_detector::ASTDependencyDetector;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
};
use std::collections::{BTreeMap, HashMap};
use txtx_addon_kit::channel;
use txtx_addon_kit::indexmap::indexmap;
use txtx_addon_kit::types::commands::{
    CommandInputsEvaluationResult, InputsPostProcessingFutureResult,
};
use txtx_addon_kit::types::types::ObjectProperty;
use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::BlockEvent,
        signers::{
            SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
        },
        types::{RunbookSupervisionContext, Type, Value},
        ConstructDid, ValueStore,
    },
    uuid::Uuid,
    AddonDefaults,
};

use super::deploy_contract::StacksDeployContract;
use crate::rpc::StacksRpc;
use crate::typing::STACKS_POST_CONDITIONS;

lazy_static! {
    pub static ref DEPLOY_STACKS_REQUIREMENT: PreCommandSpecification = {
        let mut command = define_command! {
        StacksDeployContractRequirement => {
            name: "Stacks Contract Requirement Deployment",
            matcher: "deploy_requirement",
            documentation: "The `deploy_requirement` action retrieves a deployed contract along with its dependencies, signs the transactions using a signer, and broadcasts the signed transactions to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the deployment",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                contract_id: {
                    documentation: "The contract id deployed on Mainnet that needs to mirrored.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                network_id: {
                    documentation: "The network id used to validate the transaction version.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                rpc_api_url_source: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                signer: {
                    documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                nonce: {
                    documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                fee: {
                    documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                post_conditions: {
                    documentation: "The post conditions to include to the transaction.",
                    typing: Type::array(Type::addon(STACKS_POST_CONDITIONS)),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                transforms: {
                    documentation: "An array of transform operations to perform on the contract source, before being its signature.",
                    typing: Type::array(Type::object(vec![
                        ObjectProperty {
                            name: "type".into(),
                            documentation: "Type of transform (supported: 'contract_source_find_and_replace').".into(),
                            typing: Type::string(),
                            optional: false,
                            tainting: true,
                            internal: false,
                        },
                        ObjectProperty {
                            name: "from".into(),
                            documentation: "The pattern to locate.".into(),
                            typing: Type::string(),
                            optional: false,
                            tainting: true,
                            internal: false,
                        },
                        ObjectProperty {
                            name: "to".into(),
                            documentation: "The update.".into(),
                            typing: Type::string(),
                            optional: false,
                            tainting: true,
                            internal: false,
                        }, ])),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                contracts_ids_dependencies: {
                    documentation: "Contracts that are depending on this contract at their deployment.",
                    typing: Type::array(Type::string()),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                contracts_ids_lazy_dependencies: {
                    documentation: "Contracts that are depending on this contract after their deployment.",
                    typing: Type::array(Type::string()),
                    optional: true,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
                signed_transaction_bytes: {
                    documentation: "The signed transaction bytes.",
                    typing: Type::string()
                },
                tx_id: {
                    documentation: "The transaction id.",
                    typing: Type::string()
                },
                result: {
                    documentation: "The transaction result.",
                    typing: Type::buffer()
                }
                ],
                example: txtx_addon_kit::indoc! {r#"
                        action "counter_deployment" "stacks::deploy_requirement" {
                            description = "Deploy counter contract."
                            source_code = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                            contract_name = "verify-and-update-price-feeds"
                            signer = signer.alice
                        }
                        output "contract_tx_id" {
                        value = action.counter_deployment.tx_id
                        }
                        // > contract_tx_id: 0x1020321039120390239103193012909424854753848509019302931023849320
                    "#},
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("source".to_string());
        }
        command
    };
}

pub struct StacksDeployContractRequirement;
impl CommandImplementation for StacksDeployContractRequirement {
    #[cfg(not(feature = "wasm"))]
    fn post_process_evaluated_inputs(
        _ctx: &CommandSpecification,
        mut evaluated_inputs: CommandInputsEvaluationResult,
    ) -> InputsPostProcessingFutureResult {
        let contract_id = evaluated_inputs.inputs.get_expected_string("contract_id")?;

        let rpc_api_url_source =
            evaluated_inputs.inputs.get_expected_string("rpc_api_url_source")?.to_string();

        let contract_id = QualifiedContractIdentifier::parse(contract_id)
            .map_err(|e| diagnosed_error!("unable to parse contract_id ({})", e.to_string()))?;

        let transforms = match evaluated_inputs.inputs.get_expected_array("transforms") {
            Ok(value) => value.clone(),
            Err(_) => vec![],
        };

        let future = async move {
            // Load cached contracts if existing
            // TODO

            // Fetch remote otherwise
            let client = StacksRpc::new(&rpc_api_url_source, None);
            let res = client
                .get_contract_source(&contract_id.issuer.to_string(), &contract_id.name.to_string())
                .await;
            let deployed_contract = match res {
                Ok(contract) => contract,
                Err(e) => {
                    return Err(diagnosed_error!(
                        "unable to retrieve requirement ({})",
                        e.to_string()
                    ))
                }
            };
            let clarity_version = ClarityVersion::latest();
            let interpreter =
                ClarityInterpreter::new(contract_id.issuer.clone(), Settings::default());
            let boot_contract = ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(
                    deployed_contract.source.to_string(),
                ),
                deployer: ContractDeployer::Address(contract_id.issuer.to_address()),
                name: contract_id.name.to_string(),
                epoch: StacksEpochId::latest(),
                clarity_version: clarity_version.clone(),
            };
            let (ast, _, _) = interpreter.build_ast(&boot_contract);
            let mut contracts_asts = BTreeMap::new();
            contracts_asts.insert(contract_id.clone(), (clarity_version.clone(), ast));
            let preloaded = BTreeMap::new();

            // The actual graph will be built later on, we're only using the ASTDependencyDetector to parse
            // and retrieve the dependencies.
            let mut dependencies = vec![];
            let mut lazy_dependencies = vec![];
            if let Err((data, _)) =
                ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded)
            {
                for (_contract_id, deps) in data.iter() {
                    for dep in deps.iter() {
                        let contract_id = Value::string(dep.contract_id.to_string());
                        if dep.required_before_publish {
                            dependencies.push(contract_id);
                        } else {
                            lazy_dependencies.push(contract_id);
                        }
                    }
                }
            }

            let mut contract_source = deployed_contract.source.clone();

            for transform in transforms.iter() {
                let Value::Object(props) = transform else {
                    return Err(diagnosed_error!(
                        "unable to read transform '{}'",
                        transform.to_string()
                    ));
                };

                match props.get("type") {
                    Some(Value::String(transform_type))
                        if transform_type.eq("contract_source_find_and_replace") => {}
                    _ => {
                        return Err(diagnosed_error!("transform type unsupported"));
                    }
                }

                let from = match props.get("from") {
                    Some(Value::String(from_value)) => from_value,
                    _ => {
                        return Err(diagnosed_error!("missing attribute 'from'"));
                    }
                };
                let to = match props.get("to") {
                    Some(Value::String(to_value)) => to_value,
                    _ => {
                        return Err(diagnosed_error!("missing attribute 'to'"));
                    }
                };

                contract_source = contract_source.replace(from, to);
            }

            evaluated_inputs.inputs.insert(
                "contract",
                Value::object(indexmap! {
                    "contract_source".to_string() => Value::string(contract_source),
                    "contract_name".to_string() => Value::string(contract_id.name.to_string()),
                    "clarity_version".to_string() => Value::integer(2),
                }),
            );

            evaluated_inputs
                .inputs
                .insert("contract_instance_name", Value::string(contract_id.name.to_string()));
            evaluated_inputs.inputs.insert("contract_id", Value::string(contract_id.to_string()));
            evaluated_inputs
                .inputs
                .insert("contracts_ids_dependencies", Value::array(dependencies));
            evaluated_inputs
                .inputs
                .insert("contracts_ids_lazy_dependencies", Value::array(lazy_dependencies));
            Ok(evaluated_inputs)
        };
        Ok(Box::pin(future))
    }

    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerActionsFutureResult {
        StacksDeployContract::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            defaults,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        StacksDeployContract::run_signed_execution(
            construct_did,
            spec,
            args,
            defaults,
            progress_tx,
            signers_instances,
            signers,
        )
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        StacksDeployContract::build_background_task(
            &construct_did,
            &spec,
            &inputs,
            &outputs,
            &defaults,
            &progress_tx,
            &background_tasks_uuid,
            &supervision_context,
        )
    }
}