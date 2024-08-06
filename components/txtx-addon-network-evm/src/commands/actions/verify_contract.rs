use foundry_block_explorers::Client as BlockExplorerClient;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusUpdate,
};
use txtx_addon_kit::types::types::{PrimitiveValue, RunbookSupervisionContext};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

lazy_static! {
    pub static ref VERIFY_CONTRACT: PreCommandSpecification = define_command! {
        VerifyEVMContract => {
            name: "Broadcast Stacks Transaction",
            matcher: "verify_contract",
            documentation: "The `evm::verify_contract` action sends the required contract deployment artifacts to a block explorer to verify the contract with the explorer.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                block_explorer_api_key: {
                  documentation: "The URL of the block explorer used to verify the contract.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
                },
                contract_address: {
                  documentation: "The contract address to verify.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
                },
                artifacts: {
                    documentation: indoc!{ r#"An object containing the deployment artifacts. Schema:
                    ```json
                        {
                            "abi": String,
                            "bytecode": String,
                            "source": String,
                            "compiler_version": String,
                            "contract_name": String,
                            "optimizer_enabled": Bool,
                            "optimizer_runs": UInt,
                            "evn_version": String
                        }
                    ```
                    "# },
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone(),
                    optional: false,
                    interpolable: true
                }
            ],
            outputs: [
              result: {
                  documentation: "The contract verification result.",
                  typing: Type::buffer()
              }
            ],
            example: txtx_addon_kit::indoc! {r#"
            action "verify_contract" "evm::verify_contract" {
                contract_address = evm::address(env.MY_CONTRACT_ADDRESS)
                artifacts = action.artifacts
            }
        "#},
        }
    };
}
pub struct VerifyEVMContract;
impl CommandImplementation for VerifyEVMContract {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        //    Todo: check network consistency?
        // let network = match transaction.version {
        //     TransactionVersion::Mainnet => "mainnet".to_string(),
        //     TransactionVersion::Testnet => "testnet".to_string(),
        // };

        // let network_id = args.get("network_id")
        //     .and_then(|a| Some(a.expect_string()))
        //     .or(defaults.keys.get("network_id").map(|x| x.as_str()))
        //     .ok_or(Diagnostic::error_from_string(format!("Key 'network_id' is missing")))?;
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
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    #[cfg(not(feature = "wasm"))]
    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        _outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        use alloy::{dyn_abi::DynSolValue, hex};
        use alloy_chains::Chain;
        use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
        use txtx_addon_kit::types::frontend::ProgressBarStatusColor;

        use crate::{
            codec::value_to_sol_value,
            commands::actions::get_expected_address,
            constants::{
                ARTIFACTS, BLOCK_EXPLORER_API_KEY, CHAIN_ID, CONTRACT_ADDRESS,
                CONTRACT_CONSTRUCTOR_ARGS,
            },
        };

        let inputs = inputs.clone();
        let construct_did = construct_did.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        // let network_id = inputs.get_defaulting_string(NETWORK_ID, &defaults)?;
        let chain_id = inputs.get_defaulting_uint(CHAIN_ID, &defaults)?;

        let contract_address = inputs.get_expected_value(CONTRACT_ADDRESS)?;
        let explorer_api_key = inputs.get_defaulting_string(BLOCK_EXPLORER_API_KEY, defaults)?;
        let artifacts = inputs.get_expected_object(ARTIFACTS)?;

        let constructor_args =
            if let Some(function_args) = inputs.get_value(CONTRACT_CONSTRUCTOR_ARGS) {
                let sol_args = function_args
                    .expect_array()
                    .iter()
                    .map(|v| {
                        value_to_sol_value(&v)
                            .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {}", e))
                    })
                    .collect::<Result<Vec<DynSolValue>, Diagnostic>>()?
                    .iter()
                    .flat_map(|s| s.abi_encode())
                    .collect::<Vec<u8>>();
                Some(hex::encode(&sol_args))
            } else {
                None
            };

        let contract_address = get_expected_address(&contract_address).map_err(|e| {
            diagnosed_error!("command 'evm::verify_contract' failed to parse contract address: {e}")
        })?;

        let progress_tx = progress_tx.clone();
        let progress_symbol = ["|", "/", "-", "\\", "|", "/", "-", "\\"];
        let future = async move {
            let mut progress = 0;
            let mut status_update = ProgressBarStatusUpdate::new(
                &background_tasks_uuid,
                &construct_did,
                &ProgressBarStatus {
                    status_color: ProgressBarStatusColor::Yellow,
                    status: format!("Pending {}", progress_symbol[progress]),
                    message: "".into(),
                    diagnostic: None,
                },
            );

            let mut result = CommandExecutionResult::new();

            progress = (progress + 1) % progress_symbol.len();
            status_update.update_status(&ProgressBarStatus::new_msg(
                ProgressBarStatusColor::Yellow,
                &format!("Pending {}", progress_symbol[progress]),
                &format!(
                    "Submitting Contract {} to Explorer for Verification",
                    contract_address.to_string()
                ),
            ));
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let source = get_expected_string_from_map(&artifacts, "source")?;
            let compiler_version = get_expected_string_from_map(&artifacts, "compiler_version")?;
            let contract_name = get_expected_string_from_map(&artifacts, "contract_name")?;
            let evm_version = get_expected_string_from_map(&artifacts, "evm_version")?;
            let optimizer_enabled = get_expected_bool_from_map(&artifacts, "optimizer_enabled")?;
            let optimizer_runs: u32 = get_expected_uint_from_map(&artifacts, "optimizer_runs")?
                .try_into()
                .map_err(|e| {
                    diagnosed_error!(
                        "command 'evm::verify_contract': invalid number of optimizer runs: {e}"
                    )
                })?;

            let chain = Chain::from(chain_id);
            let explorer_client = BlockExplorerClient::new(chain, explorer_api_key)
                .map_err(|e| diagnosed_error!("command 'evm::verify_contract': failed to create block explorer client: {e}"))?;

            let guid = {
                progress = (progress + 1) % progress_symbol.len();
                let verify_contract = VerifyContract::new(
                    contract_address,
                    contract_name.clone(),
                    source.clone(),
                    compiler_version.clone(),
                )
                // todo: need to check if other formats
                .code_format(CodeFormat::SingleFile)
                // todo: need to set from compilation settings
                .optimization(optimizer_enabled)
                .runs(optimizer_runs)
                .evm_version(evm_version)
                .constructor_arguments(constructor_args);

                let res = explorer_client
                    .submit_contract_verification(&verify_contract)
                    .await
                    .map_err(|e| diagnosed_error!("command 'evm::verify_contract': failed to verify contract with block explorer: {e}"))?;

                if res.message.eq("NOTOK") {
                    result
                        .outputs
                        .insert("result".into(), Value::string(res.result.clone()));

                    let diag = diagnosed_error!("command 'evm::verify_contract': failed to verify contract with block explorer: {}", res.result);
                    status_update.update_status(&ProgressBarStatus::new_err(
                        "Failed",
                        "Contract Verification Failed",
                        &diag,
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    return Err(diag);
                }
                let guid = res.result;

                status_update.update_status(&ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Yellow,
                    &format!("Pending {}", progress_symbol[progress]),
                    &format!(
                        "Contract {} Submitted to Explorer for Verification",
                        contract_address.to_string()
                    ),
                ));
                let _ =
                    progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                guid
            };

            let max_attempts = 5;
            let mut attempts = 0;
            let backoff_ms = 500;
            let status_msg = format!(
                "Checking Verification Status for Contract {}",
                contract_address.to_string()
            );
            loop {
                progress = (progress + 1) % progress_symbol.len();
                status_update.update_status(&ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Yellow,
                    &format!("Pending {}", progress_symbol[progress]),
                    &status_msg,
                ));
                let _ =
                    progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                let res = match explorer_client
                    .check_contract_verification_status(guid.clone())
                    .await
                {
                    Ok(res) => res,
                    Err(e) => {
                        attempts += 1;
                        if attempts == max_attempts {
                            let diag = diagnosed_error!("command 'evm::verify_contract': failed to verify contract with block explorer: {}", e);
                            status_update.update_status(&ProgressBarStatus::new_err(
                                "Failed",
                                "Contract Verification Failed",
                                &diag,
                            ));
                            let _ = progress_tx
                                .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                            return Err(diag);
                        } else {
                            // loop to update our progress symbol every 500ms, but still waiting 5000ms before refetching for receipt
                            let mut count = 0;
                            loop {
                                count += 1;
                                progress = (progress + 1) % progress_symbol.len();

                                status_update.update_status(&ProgressBarStatus::new_msg(
                                    ProgressBarStatusColor::Yellow,
                                    &format!("Pending {}", progress_symbol[progress]),
                                    &status_msg,
                                ));
                                let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(
                                    status_update.clone(),
                                ));

                                sleep_ms(backoff_ms);
                                if count == 10 {
                                    break;
                                }
                            }
                            continue;
                        }
                    }
                };

                if res.message.eq("NOTOK") {
                    result
                        .outputs
                        .insert("result".into(), Value::string(res.result.clone()));

                    if res.result.eq("Already Verified") {
                        status_update.update_status(&ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Verified",
                            &format!("Contract {} Already Verified", contract_address.to_string()),
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        return Ok(result);
                    } else {
                        attempts += 1;
                        if attempts == max_attempts {
                            let diag = diagnosed_error!("command 'evm::verify_contract': received error response from contract verification: {}", res.result);
                            status_update.update_status(&ProgressBarStatus::new_err(
                                "Failed",
                                "Contract Verification Failed",
                                &diag,
                            ));
                            let _ = progress_tx
                                .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                            return Err(diag);
                        }
                    }
                }

                let contract_verified = res.result;

                if contract_verified.eq("Pass - Verified") {
                    result
                        .outputs
                        .insert("result".into(), Value::string(contract_verified));

                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Green,
                        "Verified",
                        &format!("Contract {} Verified", contract_address.to_string()),
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                    break;
                } else {
                    // loop to update our progress symbol every 500ms, but still waiting 5000ms before refetching for receipt
                    let mut count = 0;
                    loop {
                        count += 1;
                        progress = (progress + 1) % progress_symbol.len();

                        status_update.update_status(&ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Yellow,
                            &format!("Pending {}", progress_symbol[progress]),
                            &status_msg,
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                        sleep_ms(backoff_ms);
                        if count == 10 {
                            result
                                .outputs
                                .insert("result".into(), Value::string(contract_verified));
                            break;
                        }
                    }
                    continue;
                }
            }

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}

pub fn get_expected_string_from_map(
    map: &IndexMap<String, Value>,
    key: &str,
) -> Result<String, Diagnostic> {
    let Some(Value::Primitive(PrimitiveValue::String(val))) = map.get(key) else {
        return Err(diagnosed_error!(
            "command 'evm::verify_contract': contract deployment artifacts missing {key}"
        ));
    };
    Ok(val.into())
}
pub fn get_expected_uint_from_map(
    map: &IndexMap<String, Value>,
    key: &str,
) -> Result<u64, Diagnostic> {
    let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) = map.get(key) else {
        return Err(diagnosed_error!(
            "command 'evm::verify_contract': contract deployment artifacts missing {key}"
        ));
    };
    Ok(*val)
}
pub fn get_expected_bool_from_map(
    map: &IndexMap<String, Value>,
    key: &str,
) -> Result<bool, Diagnostic> {
    let Some(Value::Primitive(PrimitiveValue::Bool(val))) = map.get(key) else {
        return Err(diagnosed_error!(
            "command 'evm::verify_contract': contract deployment artifacts missing {key}"
        ));
    };
    Ok(*val)
}
