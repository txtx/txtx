use foundry_block_explorers::Client as BlockExplorerClient;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusUpdate,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::constants::{DEFAULT_CONFIRMATIONS_NUMBER, RPC_API_URL};
use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

lazy_static! {
    pub static ref VERIFY_CONTRACT_DEPLOYMENT: PreCommandSpecification = define_command! {
        VerifyContractDeployment => {
            name: "Broadcast Stacks Transaction",
            matcher: "verify_deployment",
            documentation: "Coming soon",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                tx_hash: {
                  documentation: "",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
                },
                rpc_api_url: {
                  documentation: "The URL of the EVM API used to get the transaction receipt.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                },
                block_explorer_api_key: {
                  documentation: "The URL of the block explorer used to verify the contract.",
                  typing: Type::string(),
                  optional: true,
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
                            "contract_name": String
                        }
                    ```
                    "# },
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone(),
                    optional: true,
                    interpolable: true
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                    typing: Type::uint(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
              contract_address: {
                  documentation: "The transaction result.",
                  typing: Type::buffer()
              }
            ],
            example: txtx_addon_kit::indoc! {r#"
            // Coming soon
        "#},
        }
    };
}
pub struct VerifyContractDeployment;
impl CommandImplementation for VerifyContractDeployment {
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
        outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        use alloy_chains::Chain;
        use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
        use txtx_addon_kit::{
            hex,
            types::{frontend::ProgressBarStatusColor, types::PrimitiveValue},
        };

        use crate::{
            constants::{ARTIFACTS, BLOCK_EXPLORER_API_KEY, CHAIN_ID, NETWORK_ID},
            rpc::EVMRpc,
            typing::ETH_TX_HASH,
        };

        let inputs = inputs.clone();
        let construct_did = construct_did.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let confirmations_required = inputs
            .get_expected_uint("confirmations")
            .unwrap_or(DEFAULT_CONFIRMATIONS_NUMBER) as usize;

        let network_id = inputs.get_defaulting_string(NETWORK_ID, &defaults)?;
        let chain_id = inputs.get_defaulting_uint(CHAIN_ID, &defaults)?;

        let tx_hash = inputs.get_expected_buffer("tx_hash", &ETH_TX_HASH)?;
        let rpc_api_url = inputs.get_defaulting_string(RPC_API_URL, defaults)?;
        let explorer_api_key =
            inputs.get_optional_defaulting_string(BLOCK_EXPLORER_API_KEY, defaults)?;

        let artifacts = inputs.get_object(ARTIFACTS)?;

        if explorer_api_key.is_none() && artifacts.is_some() {
            return Err(diagnosed_error!("command 'evm::verify_deployment': cannot deploy artifacts without block explorer api key"));
        }

        let progress_tx = progress_tx.clone();
        let progress_symbol = ["|", "/", "-", "\\", "|", "/", "-", "\\"];
        let is_supervised = supervision_context.is_supervised;
        let future = async move {
            let mut progress = 0;
            let mut status_update = ProgressBarStatusUpdate::new(
                &background_tasks_uuid,
                &construct_did,
                &ProgressBarStatus {
                    status_color: ProgressBarStatusColor::Yellow,
                    status: format!("Pending {}", progress_symbol[progress]),
                    message: "Checking Contract Receipt".into(),
                    diagnostic: None,
                },
            );
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let mut result = CommandExecutionResult::new();

            let mut backoff_ms = 5000;

            let rpc = EVMRpc::new(&rpc_api_url)
                .map_err(|e| diagnosed_error!("command 'evm::verify_deployment': {e}"))?;

            let mut included_block = u64::MAX - confirmations_required as u64;
            let mut latest_block = 0;
            let receipt = loop {
                progress = (progress + 1) % progress_symbol.len();

                let Some(receipt) = rpc
                    .get_receipt(&tx_hash.bytes)
                    .await
                    .map_err(|e| diagnosed_error!("command 'evm::verify_deployment': {e}"))?
                else {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        "Checking Contract Receipt",
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                    sleep_ms(backoff_ms);
                    continue;
                };
                backoff_ms = 500;
                let Some(block_number) = receipt.block_number else {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        "Checking for Contract Inclusion",
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    sleep_ms(backoff_ms);
                    continue;
                };
                if latest_block == 0 {
                    included_block = block_number;
                    latest_block = block_number;
                }

                let Some(contract_address) = receipt.contract_address else {
                    return Err(diagnosed_error!(
                        "command 'evm::verify_deployment': cannot verify transaction that did not deploy contract: {}",
                        hex::encode(tx_hash.bytes)
                    ));
                };

                if latest_block >= included_block + confirmations_required as u64 {
                    break receipt;
                } else {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        &format!("Waiting for {} block confirmations", confirmations_required),
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    latest_block = rpc
                        .get_block_number()
                        .await
                        .map_err(|e| diagnosed_error!("command 'evm::verify_deployment': {e}"))?;
                    sleep_ms(backoff_ms);
                    continue;
                }
            };

            if let Some(artifacts) = artifacts {
                progress = (progress + 1) % progress_symbol.len();
                status_update.update_status(&ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Yellow,
                    &format!("Pending {}", progress_symbol[progress]),
                    "Submitting to Explorer for Verification",
                ));
                let _ =
                    progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                let Some(Value::Primitive(PrimitiveValue::String(source))) =
                    artifacts.get("source")
                else {
                    return Err(diagnosed_error!(
                        "command: 'evm::verify_deployment': contract deployment artifacts missing contract source"
                    ));
                };

                let Some(Value::Primitive(PrimitiveValue::String(compiler_version))) =
                    artifacts.get("compiler_version")
                else {
                    return Err(diagnosed_error!(
                        "command: 'evm::verify_deployment': contract deployment artifacts missing compiler version"
                    ));
                };

                let Some(Value::Primitive(PrimitiveValue::String(contract_name))) =
                    artifacts.get("contract_name")
                else {
                    return Err(diagnosed_error!(
                        "command: 'evm::verify_deployment': contract deployment artifacts missing contract name"
                    ));
                };

                let chain = Chain::from(chain_id);
                let explorer_client = BlockExplorerClient::new(chain, explorer_api_key.unwrap())
                .map_err(|e| diagnosed_error!("command 'evm::verify_deployment': failed to create block explorer client: {e}"))?;

                let guid = {
                    progress = (progress + 1) % progress_symbol.len();
                    let verify_contract = VerifyContract::new(
                        receipt.contract_address.unwrap(),
                        contract_name.clone(),
                        source.clone(),
                        compiler_version.clone(),
                    )
                    // todo: need to check if other formats
                    .code_format(CodeFormat::SingleFile)
                    // todo: need to set from compilation settings
                    .optimization(true)
                    .runs(200)
                    .evm_version("paris");

                    let res = explorer_client
                    .submit_contract_verification(&verify_contract)
                    .await
                    .map_err(|e| diagnosed_error!("command 'evm::verify_deployment': failed to verify contract with block explorer: {e}"))?;

                    if res.message.eq("NOTOK") {
                        let diag = diagnosed_error!("command 'evm::verify_deployment': failed to verify contract with block explorer: {}", res.result);
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
                        "Submitted to Explorer for Verification",
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                    println!("guid: {guid}");
                    guid
                };

                let max_attempts = 5;
                let mut attempts = 0;
                loop {
                    progress = (progress + 1) % progress_symbol.len();
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        "Checking Contract Verification Status",
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    let res = match explorer_client
                        .check_contract_verification_status(guid.clone())
                        .await
                    {
                        Ok(res) => res,
                        Err(e) => {
                            attempts += 1;
                            if attempts == max_attempts {
                                let diag = diagnosed_error!("command 'evm::verify_deployment': failed to verify contract with block explorer: {}", e);
                                status_update.update_status(&ProgressBarStatus::new_err(
                                    "Failed",
                                    "Contract Verification Failed",
                                    &diag,
                                ));
                                let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(
                                    status_update.clone(),
                                ));

                                return Err(diag);
                            } else {
                                sleep_ms(5000);
                                continue;
                            }
                        }
                    };

                    if res.message.eq("NOTOK") {
                        attempts += 1;
                        if attempts == max_attempts {
                            let diag = diagnosed_error!("command 'evm::verify_deployment': received error response from contract verification: {}", res.result);
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

                    let contract_verified = res.result;

                    if contract_verified.eq("Pass - Verified") {
                        break;
                    } else {
                        println!("contract verified status: {}", contract_verified);
                        sleep_ms(5000);
                    }
                }
            }

            status_update.update_status(&ProgressBarStatus::new_msg(
                ProgressBarStatusColor::Green,
                "Confirmed",
                "Transaction Receipt Found",
            ));
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
            println!(
                "contract address for deployed contract {:?}",
                receipt.contract_address.unwrap()
            );
            result.outputs.insert(
                "contract_address".into(),
                Value::string(receipt.contract_address.unwrap().to_string()),
            );

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}
