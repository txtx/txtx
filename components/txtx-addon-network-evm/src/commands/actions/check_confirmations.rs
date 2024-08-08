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

lazy_static! {
    pub static ref CHECK_CONFIRMATIONS: PreCommandSpecification = define_command! {
        CheckEVMConfirmations => {
            name: "Check Transaction Confirmations",
            matcher: "check_confirmations",
            documentation: "The `evm::check_confirmations` action polls the network until the provided `tx_hash` has been confirmed by `confirmations` blocks.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                tx_hash: {
                  documentation: "The transaction hash to check.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
                },
                rpc_api_url: {
                  documentation: "The URL of the EVM API used to poll for the transaction's inclusion in a block.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
                },
                chain_id: {
                  documentation: "Coming soon",
                  typing: Type::uint(),
                  optional: false,
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
                    documentation: "The contract address from the transaction receipt.",
                    typing: Type::buffer()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
            action "confirm_deployment" "evm::check_confirmations" {
                tx_hash = action.some_deploying_action.tx_hash
                confirmations = 1
            }
        "#},
        }
    };
}
pub struct CheckEVMConfirmations;
impl CommandImplementation for CheckEVMConfirmations {
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
        use alloy_chains::{Chain, ChainKind};
        use txtx_addon_kit::{
            hex,
            types::{commands::return_synchronous_result, frontend::ProgressBarStatusColor},
        };

        use crate::{
            constants::{ALREADY_DEPLOYED, CHAIN_ID, CONTRACT_ADDRESS, TX_HASH},
            rpc::EVMRpc,
            typing::ETH_TX_HASH,
        };

        let inputs = inputs.clone();
        let construct_did = construct_did.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let confirmations_required = inputs
            .get_expected_uint("confirmations")
            .unwrap_or(DEFAULT_CONFIRMATIONS_NUMBER) as usize;
        let chain_id = inputs.get_defaulting_uint(CHAIN_ID, &defaults)?;
        let chain_name = match Chain::from(chain_id).into_kind() {
            ChainKind::Named(name) => name.to_string(),
            ChainKind::Id(id) => id.to_string(),
        };
        let progress_tx = progress_tx.clone();

        let skip_confirmations = inputs.get_bool(ALREADY_DEPLOYED).unwrap_or(false);
        let contract_address = inputs.get_value(CONTRACT_ADDRESS);
        if skip_confirmations {
            let mut result = CommandExecutionResult::new();
            if let Some(contract_address) = contract_address {
                result
                    .outputs
                    .insert(CONTRACT_ADDRESS.to_string(), contract_address.clone());
            }

            let status_update = ProgressBarStatusUpdate::new(
                &background_tasks_uuid,
                &construct_did,
                &ProgressBarStatus {
                    status_color: ProgressBarStatusColor::Green,
                    status: format!("Confirmed"),
                    message: format!(
                        "Contract deployment transaction already confirmed on Chain {}",
                        chain_name
                    ),
                    diagnostic: None,
                },
            );
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
            return return_synchronous_result(Ok(result));
        }

        let tx_hash = inputs.get_expected_buffer(TX_HASH, &ETH_TX_HASH)?;
        let rpc_api_url = inputs.get_defaulting_string(RPC_API_URL, defaults)?;

        let progress_symbol = ["|", "/", "-", "\\", "|", "/", "-", "\\"];

        let tx_hash_str = hex::encode(tx_hash.bytes.clone());
        let receipt_msg = format!(
            "Checking Tx 0x{} Receipt on Chain {}",
            &tx_hash_str, chain_name
        );

        let future = async move {
            // initial progress status
            let mut progress = 0;
            let mut status_update = ProgressBarStatusUpdate::new(
                &background_tasks_uuid,
                &construct_did,
                &ProgressBarStatus {
                    status_color: ProgressBarStatusColor::Yellow,
                    status: format!("Pending {}", progress_symbol[progress]),
                    message: receipt_msg.clone(),
                    diagnostic: None,
                },
            );
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let mut result = CommandExecutionResult::new();

            let backoff_ms = 500;

            let rpc = EVMRpc::new(&rpc_api_url)
                .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {e}"))?;

            let mut included_block = u64::MAX - confirmations_required as u64;
            let mut latest_block = 0;
            let _receipt = loop {
                progress = (progress + 1) % progress_symbol.len();

                let Some(receipt) = rpc
                    .get_receipt(&tx_hash.bytes)
                    .await
                    .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {e}"))?
                else {
                    // loop to update our progress symbol every 500ms, but still waiting 5000ms before refetching for receipt
                    let mut count = 0;
                    loop {
                        count += 1;
                        progress = (progress + 1) % progress_symbol.len();
                        status_update.update_status(&ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Yellow,
                            &format!("Pending {}", progress_symbol[progress]),
                            &receipt_msg,
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        sleep_ms(backoff_ms);
                        if count == 10 {
                            break;
                        }
                    }
                    continue;
                };
                let Some(block_number) = receipt.block_number else {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        &format!(
                            "Awaiting Inclusion in Block for Tx 0x{} on Chain {}",
                            tx_hash_str, chain_name
                        ),
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

                if !receipt.status() {
                    let diag = diagnosed_error!("transaction reverted");
                    status_update.update_status(&ProgressBarStatus::new_err(
                        "Failed",
                        &format!("Transaction Failed for Chain {}", chain_name),
                        &diag,
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    return Err(diag);
                }
                if let Some(contract_address) = receipt.contract_address {
                    result.outputs.insert(
                        CONTRACT_ADDRESS.to_string(),
                        Value::string(contract_address.to_string()),
                    );
                };

                if latest_block >= included_block + confirmations_required as u64 {
                    break receipt;
                } else {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        &format!(
                            "Waiting for {} Block Confirmations on Chain {}",
                            confirmations_required, chain_name
                        ),
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    latest_block = rpc
                        .get_block_number()
                        .await
                        .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {e}"))?;
                    sleep_ms(backoff_ms);
                    continue;
                }
            };

            status_update.update_status(&ProgressBarStatus::new_msg(
                ProgressBarStatusColor::Green,
                "Confirmed",
                &format!(
                    "Confirmed {} {} for Tx 0x{} on Chain {}",
                    &confirmations_required,
                    if confirmations_required.eq(&1) {
                        "block"
                    } else {
                        "blocks"
                    },
                    tx_hash_str,
                    chain_name
                ),
            ));
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}
