use clarity::types::StacksEpochId;
use clarity::vm::ast::ContractAST;
use clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::{ClarityVersion, ContractName};
use clarity_repl::analysis::ast_dependency_detector::ASTDependencyDetector;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
};
use std::collections::{BTreeMap, HashMap};
use std::future;
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
        types::{RunbookSupervisionContext, Type, Value},
        wallets::{
            SigningCommandsState, WalletActionsFutureResult, WalletInstance, WalletSignFutureResult,
        },
        ConstructDid, ValueStore,
    },
    uuid::Uuid,
    AddonDefaults,
};

use crate::{
    constants::{
        SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES, TRANSACTION_POST_CONDITIONS_BYTES,
    },
    typing::STACKS_POST_CONDITION,
};

use super::encode_contract_deployment;
use super::{
    broadcast_transaction::BroadcastStacksTransaction, get_signing_construct_did,
    sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT_REQUIREMENT: PreCommandSpecification = {
        let mut command = define_command! {
        StacksDeployContractRequirement => {
            name: "Stacks Contract Requirement Deployment",
            matcher: "deploy_contract_requirement",
            documentation: "The `deploy_contract` action encodes a contract deployment transaction, signs the transaction using a wallet, and broadcasts the signed transaction to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the deployment",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                contract_id: {
                    documentation: "The contract id deployed on Mainnet that needs to mirrored.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                network_id: {
                    documentation: "The network id used to validate the transaction version.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                signer: {
                    documentation: "A reference to a wallet construct, which will be used to sign the transaction payload.",
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
                    typing: Type::array(Type::addon(STACKS_POST_CONDITION.clone())),
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
                        action "counter_deployment" "stacks::deploy_contract" {
                            description = "Deploy counter contract."
                            source_code = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                            contract_name = "verify-and-update-price-feeds"
                            signer = wallet.alice
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
    fn post_process_evaluated_inputs(
        _ctx: &CommandSpecification,
        mut evaluated_inputs: CommandInputsEvaluationResult,
        // ) -> InputPostProcessingFutureResult {
    ) -> Result<CommandInputsEvaluationResult, Diagnostic> {
        let contract = evaluated_inputs.inputs.get_expected_object("contract")?;
        let contract_source = match contract.get("contract_source").map(|v| v.as_string()) {
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

        evaluated_inputs
            .inputs
            .insert("contract_id", Value::string(contract_id.to_string()));
        evaluated_inputs
            .inputs
            .insert("contracts_ids_dependencies", Value::array(dependencies));
        evaluated_inputs.inputs.insert(
            "contracts_ids_lazy_dependencies",
            Value::array(lazy_dependencies),
        );

        Ok(evaluated_inputs)
        // Ok(Box::pin(future::ready(Ok(evaluated_inputs))))
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
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletActionsFutureResult {
        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let signing_command_state = wallets
            .pop_signing_command_state(&signing_construct_did)
            .unwrap();

        // Extract network_id
        let (contract_source, contract_name, clarity_version) = match args
            .get_expected_object("contract")
        {
            Ok(value) => {
                let contract_source = match value.get("contract_source").map(|v| v.as_string()) {
                    Some(Some(value)) => value.to_string(),
                    _ => {
                        return Err((
                            wallets,
                            signing_command_state,
                            diagnosed_error!("unable to retrieve 'contract_source'"),
                        ))
                    }
                };
                let contract_name = match value.get("contract_name").map(|v| v.as_string()) {
                    Some(Some(value)) => value.to_string(),
                    _ => {
                        return Err((
                            wallets,
                            signing_command_state,
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
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };

        let empty_vec = vec![];
        let post_conditions_values = args
            .get_expected_array("post_conditions")
            .unwrap_or(&empty_vec);
        let bytes = match encode_contract_deployment(
            spec,
            &contract_source,
            &contract_name,
            clarity_version,
        ) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        wallets.push_signing_command_state(signing_command_state);

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );

        SignStacksTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            defaults,
            supervision_context,
            wallets_instances,
            wallets,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletSignFutureResult {
        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let signing_command_state = wallets
            .pop_signing_command_state(&signing_construct_did)
            .unwrap();

        // Extract network_id
        let (contract_source, contract_name, clarity_version) = match args
            .get_expected_object("contract")
        {
            Ok(value) => {
                let contract_source = match value.get("contract_source").map(|v| v.as_string()) {
                    Some(Some(value)) => value.to_string(),
                    _ => {
                        return Err((
                            wallets,
                            signing_command_state,
                            diagnosed_error!("unable to retrieve 'contract_source'"),
                        ))
                    }
                };
                let contract_name = match value.get("contract_name").map(|v| v.as_string()) {
                    Some(Some(value)) => value.to_string(),
                    _ => {
                        return Err((
                            wallets,
                            signing_command_state,
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
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        wallets.push_signing_command_state(signing_command_state);

        let empty_vec = vec![];
        let post_conditions_values = args
            .get_expected_array("post_conditions")
            .unwrap_or(&empty_vec);
        let bytes =
            encode_contract_deployment(spec, &contract_source, &contract_name, clarity_version)
                .unwrap();

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
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

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
                &wallets_instances,
                wallets,
            );
            let (wallets, signing_command_state, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            args.insert(
                SIGNED_TRANSACTION_BYTES,
                res_signing
                    .outputs
                    .get(SIGNED_TRANSACTION_BYTES)
                    .unwrap()
                    .clone(),
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
                    Err(diag) => return Err((wallets, signing_command_state, diag)),
                },
                Err(data) => return Err((wallets, signing_command_state, data)),
            };

            res_signing.append(&mut res);

            Ok((wallets, signing_command_state, res_signing))
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use clarity::vm::{
        types::{QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion, ContractName,
    };
    use clarity_repl::analysis::ast_dependency_detector::ASTDependencyDetector;

    #[test]
    fn it_retrieve_dependencies() {
        let contract_name: ContractName = "transient".try_into().unwrap();
        let sender = StandardPrincipalData::transient();
        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());
        let clarity_version = ClarityVersion::latest();

        let ast =
            super::build_ast_from_src(&sender, "(+ 1 1)", &contract_name, clarity_version.clone());
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
        let preloaded = BTreeMap::new();
        let res = ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded).unwrap();
        assert_eq!(res.len(), 1);

        let ast = super::build_ast_from_src(
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

        let ast = super::build_ast_from_src(&sender, "(begin (contract-call? .test-contract-1 contract-call u1) (contract-call? .test-contract-2 contract-call u1))", &contract_name, clarity_version.clone());
        let mut contracts_asts = BTreeMap::new();
        contracts_asts.insert(contract_id.clone(), (clarity_version, ast));
        let preloaded = BTreeMap::new();
        let (_, deps) =
            ASTDependencyDetector::detect_dependencies(&contracts_asts, &preloaded).unwrap_err();
        assert_eq!(deps.len(), 2);
    }
}
