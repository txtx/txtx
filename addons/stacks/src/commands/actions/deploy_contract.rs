use clarity::codec::StacksMessageCodec;
use clarity::types::StacksEpochId;
use clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::{ClarityVersion, ContractName};
use clarity_repl::analysis::ast_dependency_detector::ASTDependencyDetector;
use clarity_repl::codec::StacksTransaction;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
};
use std::collections::{BTreeMap, HashMap};
use txtx_addon_kit::channel;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
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

use crate::constants::TRANSACTION_POST_CONDITION_MODE_BYTES;
use crate::{
    constants::{
        SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES, TRANSACTION_POST_CONDITIONS_BYTES,
    },
    typing::STACKS_POST_CONDITIONS,
};

use super::encode_contract_deployment;
use super::{
    broadcast_transaction::BroadcastStacksTransaction, get_signer_did,
    sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: PreCommandSpecification = {
        let mut command = define_command! {
        StacksDeployContract => {
            name: "Stacks Contract Deployment",
            matcher: "deploy_contract",
            documentation: "The `deploy_contract` action encodes a contract deployment transaction, signs the transaction using a signer, and broadcasts the signed transaction to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the deployment",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                contract: {
                    documentation: "Contract informations.",
                    typing: Type::object(vec![
                        ObjectProperty {
                            name: "source_code".into(),
                            documentation: "The code of the contract method to deploy.".into(),
                            typing: Type::string(),
                            optional: true,
                            interpolable: true,
                        },
                        ObjectProperty {
                            name: "contract_name".into(),
                            documentation: "The name of the contract to deploy.".into(),
                            typing: Type::string(),
                            optional: true,
                            interpolable: true,
                        },
                        ObjectProperty {
                            name: "clarity_version".into(),
                            documentation: "The version of clarity to use (default: latest).".into(),
                            typing: Type::integer(),
                            optional: true,
                            interpolable: true,
                        }, ]),
                    optional: true,
                    interpolable: true
                },
                network_id: {
                    documentation: "The network id used to validate the transaction version.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                signer: {
                    documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                    typing: Type::integer(),
                    optional: true,
                    interpolable: true
                },
                nonce: {
                    documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    interpolable: true
                },
                fee: {
                    documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    interpolable: true
                },
                post_conditions: {
                    documentation: "The post conditions to include to the transaction.",
                    typing: Type::array(Type::addon(STACKS_POST_CONDITIONS)),
                    optional: true,
                    interpolable: true
                  },
                post_condition_mode: {
                    documentation: "The post condition mode ('allow', 'deny'). In Allow mode other asset transfers not covered by the post-conditions are permitted. In Deny mode no other asset transfers are permitted besides those named in the post-conditions.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                  },
                transforms: {
                    documentation: "An array of transform operations to perform on the contract source, before being its signature.",
                    typing: Type::array(Type::object(vec![
                        ObjectProperty {
                            name: "type".into(),
                            documentation: "Type of transform (supported: 'contract_source_find_and_replace').".into(),
                            typing: Type::string(),
                            optional: false,
                            interpolable: true,
                        },
                        ObjectProperty {
                            name: "from".into(),
                            documentation: "The pattern to locate.".into(),
                            typing: Type::string(),
                            optional: false,
                            interpolable: true,
                        },
                        ObjectProperty {
                            name: "to".into(),
                            documentation: "The update.".into(),
                            typing: Type::string(),
                            optional: false,
                            interpolable: true,
                        }, ])),
                    optional: true,
                    interpolable: true
                },
                contracts_ids_dependencies: {
                    documentation: "Contracts that are depending on this contract at their deployment.",
                    typing: Type::array(Type::string()),
                    optional: true,
                    interpolable: true
                },
                contracts_ids_lazy_dependencies: {
                    documentation: "Contracts that are depending on this contract after their deployment.",
                    typing: Type::array(Type::string()),
                    optional: true,
                    interpolable: true
                },
                fee_strategy: {
                    documentation: "The strategy to use for automatically estimating fee ('low', 'medium', 'high'). Default to 'medium'.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
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
                contract_id: {
                    documentation: "The contract id.",
                    typing: Type::string()
                },
                result: {
                    documentation: "The transaction result.",
                    typing: Type::buffer()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                    action "counter_deployment" "stacks::deploy_contract" {
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

#[allow(dead_code)]
pub enum ContractSourceTransformsApplied {
    FindAndReplace(String, String),
}

pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
    fn post_process_evaluated_inputs(
        _ctx: &CommandSpecification,
        mut evaluated_inputs: CommandInputsEvaluationResult,
        // ) -> InputPostProcessingFutureResult {
    ) -> Result<CommandInputsEvaluationResult, Diagnostic> {
        let contract = evaluated_inputs.inputs.get_expected_object("contract")?;
        let mut contract_source = match contract.get("contract_source").map(|v| v.as_string()) {
            Some(Some(value)) => value.to_string(),
            _ => return Err(diagnosed_error!("unable to retrieve 'contract_source'")),
        };
        let contract_name: ContractName = match contract.get("contract_name").map(|v| v.as_string())
        {
            Some(Some(value)) => ContractName::try_from(value)
                .map_err(|e| diagnosed_error!("invalid contract_name: {}", e.to_string()))?,
            _ => return Err(diagnosed_error!("unable to retrieve 'contract_name'")),
        };
        let clarity_version = match contract.get("clarity_version").map(|v| v.as_uint()) {
            Some(Some(Ok(1))) => ClarityVersion::Clarity1,
            Some(Some(Ok(2))) => ClarityVersion::Clarity2,
            _ => ClarityVersion::latest(),
        };

        // TODO: Generate a hash of the signer construct_did instead.
        let transient = StandardPrincipalData::transient();
        let contract_id =
            QualifiedContractIdentifier::new(transient.clone(), contract_name.clone());
        let interpreter = ClarityInterpreter::new(transient.clone(), Settings::default());
        let boot_contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(contract_source.to_string()),
            deployer: ContractDeployer::Address(transient.to_address()),
            name: contract_name.to_string(),
            epoch: StacksEpochId::latest(),
            clarity_version: clarity_version.clone(),
        };
        let (ast, _, _) = interpreter.build_ast(&boot_contract);
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
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

        // Dependencies muts be identified before applying the contract_source_transforms
        evaluated_inputs.inputs.insert("contract_id", Value::string(contract_id.to_string()));
        evaluated_inputs.inputs.insert("contracts_ids_dependencies", Value::array(dependencies));
        evaluated_inputs
            .inputs
            .insert("contracts_ids_lazy_dependencies", Value::array(lazy_dependencies));

        // contract_source_transforms_handling.
        let mut transforms_applied = vec![];
        if let Ok(transforms) = evaluated_inputs.inputs.get_expected_array("transforms") {
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
                transforms_applied.push(ContractSourceTransformsApplied::FindAndReplace(
                    from.to_string(),
                    to.to_string(),
                ));
            }
        }

        evaluated_inputs
            .inputs
            .insert("contract_source_post_transforms", Value::string(contract_source));

        Ok(evaluated_inputs)
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
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        // Extract network_id
        let (contract_source, contract_name, clarity_version) =
            match args.get_expected_object("contract") {
                Ok(value) => {
                    let contract_source = match args
                        .get_value("contract_source_post_transforms")
                        .or(value.get("contract_source"))
                        .map(|v| v.as_string())
                    {
                        Some(Some(value)) => value.to_string(),
                        _ => {
                            return Err((
                                signers,
                                signer_state,
                                diagnosed_error!("unable to retrieve 'contract_source'"),
                            ))
                        }
                    };
                    let contract_name = match value.get("contract_name").map(|v| v.as_string()) {
                        Some(Some(value)) => value.to_string(),
                        _ => {
                            return Err((
                                signers,
                                signer_state,
                                diagnosed_error!("unable to retrieve 'contract_name'"),
                            ))
                        }
                    };
                    let clarity_version = match value.get("clarity_version").map(|v| v.as_uint()) {
                        Some(Some(Ok(value))) => Some(value),
                        _ => None,
                    };
                    (contract_source, contract_name, clarity_version)
                }
                Err(diag) => return Err((signers, signer_state, diag)),
            };

        let empty_vec: Vec<Value> = vec![];
        let post_conditions_values =
            args.get_expected_array("post_conditions").unwrap_or(&empty_vec);
        let post_condition_mode = args.get_string("post_condition_mode").unwrap_or("deny");
        let bytes = match encode_contract_deployment(
            spec,
            &contract_source,
            &contract_name,
            clarity_version,
        ) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        signers.push_signer_state(signer_state);

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );
        args.insert(
            TRANSACTION_POST_CONDITION_MODE_BYTES,
            Value::string(post_condition_mode.to_string()),
        );

        SignStacksTransaction::check_signed_executability(
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
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        // Extract network_id
        let (contract_source, contract_name, clarity_version) =
            match args.get_expected_object("contract") {
                Ok(value) => {
                    let contract_source = match args
                        .get_value("contract_source_post_transforms")
                        .or(value.get("contract_source"))
                        .map(|v| v.as_string())
                    {
                        Some(Some(value)) => value.to_string(),
                        _ => {
                            return Err((
                                signers,
                                signer_state,
                                diagnosed_error!("unable to retrieve 'contract_source'"),
                            ))
                        }
                    };
                    let contract_name = match value.get("contract_name").map(|v| v.as_string()) {
                        Some(Some(value)) => value.to_string(),
                        _ => {
                            return Err((
                                signers,
                                signer_state,
                                diagnosed_error!("unable to retrieve 'contract_name'"),
                            ))
                        }
                    };
                    let clarity_version = match value.get("clarity_version").map(|v| v.as_uint()) {
                        Some(Some(Ok(value))) => Some(value),
                        _ => None,
                    };
                    (contract_source, contract_name, clarity_version)
                }
                Err(diag) => return Err((signers, signer_state, diag)),
            };
        signers.push_signer_state(signer_state);

        let empty_vec = vec![];
        let post_conditions_values =
            args.get_expected_array("post_conditions").unwrap_or(&empty_vec);
        let post_condition_mode = args.get_string("post_condition_mode").unwrap_or("deny");
        let bytes =
            encode_contract_deployment(spec, &contract_source, &contract_name, clarity_version)
                .unwrap();

        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let defaults = defaults.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );
        args.insert(
            TRANSACTION_POST_CONDITION_MODE_BYTES,
            Value::string(post_condition_mode.to_string()),
        );

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            let signed_transaction = res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap();
            let signed_transaction_bytes = signed_transaction.clone().expect_buffer_bytes();
            let transaction =
                StacksTransaction::consensus_deserialize(&mut &signed_transaction_bytes[..])
                    .unwrap();
            let sender_address = transaction.origin_address().to_string();

            args.insert(
                SIGNED_TRANSACTION_BYTES,
                res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap().clone(),
            );

            let mut res = match BroadcastStacksTransaction::run_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((signers, signer_state, diag)),
                },
                Err(data) => return Err((signers, signer_state, data)),
            };
            res.outputs.insert(
                "contract_id".into(),
                Value::string(format!("{}.{}", sender_address.to_string(), contract_name)),
            );

            res_signing.append(&mut res);

            Ok((signers, signer_state, res_signing))
        };
        Ok(Box::pin(future))
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
        BroadcastStacksTransaction::build_background_task(
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

#[cfg(test)]
mod tests {
    use clarity::{
        types::StacksEpochId,
        vm::{
            ast::ContractAST,
            types::{QualifiedContractIdentifier, StandardPrincipalData},
            ClarityVersion, ContractName,
        },
    };
    use clarity_repl::{
        analysis::ast_dependency_detector::ASTDependencyDetector,
        repl::{
            ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
        },
    };
    use std::collections::BTreeMap;

    fn build_ast_from_src(
        sender: &StandardPrincipalData,
        contract_source: &str,
        contract_name: &ContractName,
        clarity_version: ClarityVersion,
    ) -> ContractAST {
        let interpreter = ClarityInterpreter::new(sender.clone(), Settings::default());
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(contract_source.to_string()),
            deployer: ContractDeployer::Address(sender.to_address()),
            name: contract_name.to_string(),
            epoch: StacksEpochId::latest(),
            clarity_version: clarity_version.clone(),
        };
        let (ast, _, _) = interpreter.build_ast(&contract);
        ast
    }

    #[test]
    fn it_retrieve_dependencies() {
        let contract_name: ContractName = "transient".try_into().unwrap();
        let sender = StandardPrincipalData::transient();
        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());
        let clarity_version = ClarityVersion::latest();

        let ast = build_ast_from_src(&sender, "(+ 1 1)", &contract_name, clarity_version.clone());
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
        let preloaded = BTreeMap::new();
        let res = ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded).unwrap();
        assert_eq!(res.len(), 1);

        let ast = build_ast_from_src(
            &sender,
            "(contract-call? .test-contract contract-call u1)",
            &contract_name,
            clarity_version.clone(),
        );
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
        let preloaded = BTreeMap::new();
        let (_, deps) =
            ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded).unwrap_err();
        assert_eq!(deps.len(), 1);

        let ast = build_ast_from_src(&sender, "(begin (contract-call? .test-contract-1 contract-call u1) (contract-call? .test-contract-2 contract-call u1))", &contract_name, clarity_version.clone());
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
        let preloaded = BTreeMap::new();
        let (_, deps) =
            ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded).unwrap_err();
        assert_eq!(deps.len(), 2);
    }
}
