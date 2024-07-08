use clarity::util::sleep_ms;
use std::collections::VecDeque;
use std::fmt::Write;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionFutureResult, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusUpdate,
};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::constants::{DEFAULT_CONFIRMATIONS_NUMBER, RPC_API_URL, SIGNED_TRANSACTION_BYTES};
use crate::rpc::StacksRpc;
use crate::typing::{CLARITY_BUFFER, CLARITY_VALUE};

lazy_static! {
    pub static ref BROADCAST_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
        BroadcastStacksTransaction => {
            name: "Broadcast Stacks Transaction",
            matcher: "broadcast_transaction",
            documentation: "The `broadcast_transaction` action sends a signed transaction payload to the specified network.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                signed_transaction_bytes: {
                  documentation: "The signed transaction bytes that will be broadcasted to the network.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
                },
                rpc_api_url: {
                  documentation: "The URL of the Stacks API to broadcast to.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                    typing: Type::uint(),
                    optional: true,
                    interpolable: true
                }
                // todo:
                // success_required: {
                //     documentation: "Success required.",
                //     typing: Type::bool(),
                //     optional: true,
                //     interpolable: true
                // }
            ],
            outputs: [
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
            action "my_ref" "stacks::broadcast_transaction" {
                description = "Broadcasts the signed transaction bytes"
                signed_transaction_bytes = "0x8080000000040063A5EDA39412C016478AE5A8C300843879F78245000000000000000100000000000004B0000182C1712C31B7F683F6C56EEE8920892F735FC0118C98FD10C1FDAA85ABEC2808063773E5F61229D76B29784B8BBBBAAEA72EEA701C92A4FE15EF3B9E32A373D8020100000000021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631"
            }
            output "tx_id" {
              value = action.my_ref.tx_id
            }
            output "result" {
              value = action.my_ref.result
            }
            // > tx_id: 0x...
            // > result: success
        "#},
        }
    };
}
pub struct BroadcastStacksTransaction;
impl CommandImplementation for BroadcastStacksTransaction {
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _uuid: &ConstructUuid,
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
        uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::types::frontend::ProgressBarStatusColor;

        use crate::{
            constants::NETWORK_ID, rpc::TransactionStatus, stacks_helpers::txid_display_str,
        };

        let args = args.clone();
        let uuid = uuid.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let confirmations_required = args
            .get_expected_uint("confirmations")
            .unwrap_or(DEFAULT_CONFIRMATIONS_NUMBER) as usize;

        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err(diag),
        };

        let transaction_bytes =
            args.get_expected_buffer(SIGNED_TRANSACTION_BYTES, &CLARITY_BUFFER)?;

        let rpc_api_url = args.get_defaulting_string(RPC_API_URL, defaults)?;
        let progress_tx = progress_tx.clone();
        let future = async move {
            let mut result = CommandExecutionResult::new();

            let mut s = String::from("0x");
            s.write_str(
                &transaction_bytes
                    .bytes
                    .clone()
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<String>(),
            )
            .map_err(|e| {
                Diagnostic::error_from_string(format!("Failed to serialize transaction bytes: {e}"))
            })?;

            let backoff_ms = 5000;

            let client = StacksRpc::new(&rpc_api_url);
            let mut retry_count = 4;
            let tx_result = loop {
                match client.post_transaction(&transaction_bytes.bytes).await {
                    Ok(res) => break res,
                    Err(e) => {
                        retry_count -= 1;
                        if retry_count > 0 {
                            sleep_ms(backoff_ms);
                            continue;
                        }

                        return Err(Diagnostic::error_from_string(format!(
                            "Failed to broadcast stacks transaction: {e}"
                        )));
                    }
                }
            };

            let txid = tx_result.txid;
            result
                .outputs
                .insert(format!("tx_id"), Value::string(txid.clone()));

            let moved_txid = txid.clone();
            let moved_network_id = network_id.clone();
            let wrap_msg = move |msg: &str| {
                txtx_addon_kit::formatdoc! {
                    r#"<a target="_blank" href="https://explorer.hiro.so/txid/{}?chain={}">{}</a>"#,
                    moved_txid, moved_network_id, msg
                }
                .to_string()
            };

            let progress_tx = progress_tx.clone();
            let mut retry_count = 4;
            let mut status_update = ProgressBarStatusUpdate::new(
                &background_tasks_uuid,
                &uuid.value(),
                &ProgressBarStatus {
                    status_color: ProgressBarStatusColor::Yellow,
                    status: "Pending".to_string(),
                    message: wrap_msg(&format!("Transaction 0x{}", txid_display_str(&txid))),
                    diagnostic: None,
                },
            );
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let mut block_height = 0;
            let mut confirmed_blocks_ids = VecDeque::new();
            let backoff_ms = 500;

            // let progress_symbol = ["⠁", "⠃", "⠇", "⠧", "⠷", "⠿"];
            let progress_symbol = ["|", "/", "-", "\\", "|", "/", "-", "\\"];
            let mut progress = 0;

            loop {
                progress = (progress + 1) % progress_symbol.len();

                if confirmed_blocks_ids.len() >= confirmations_required {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Green,
                        "Complete",
                        &wrap_msg(&format!("Confirmed {} blocks", &confirmations_required)),
                    ));

                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                    break;
                }

                let node_info = match client.get_info().await {
                    Ok(res) => res,
                    Err(e) => {
                        retry_count -= 1;
                        sleep_ms(backoff_ms);
                        if retry_count > 0 {
                            continue;
                        }
                        let diag = Diagnostic::error_from_string(format!(
                            "Failed to broadcast stacks transaction: {e}"
                        ));
                        status_update.update_status(&ProgressBarStatus::new_err(
                            "Failure",
                            &wrap_msg("Broadcast failed."),
                            &diag,
                        ));

                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        return Err(diag);
                    }
                };

                if node_info.stacks_tip_height == block_height {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Yellow,
                        &format!("Pending {}", progress_symbol[progress]),
                        &wrap_msg(&format!("Transaction 0x{}", txid_display_str(&txid))),
                    ));
                    let _ = progress_tx
                        .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                    // no new block
                    sleep_ms(backoff_ms);
                    continue;
                }

                block_height = node_info.stacks_tip_height;

                status_update.update_status(&ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Yellow,
                    &format!("Pending {}", progress_symbol[progress]),
                    &wrap_msg(&format!("Transaction 0x{}", txid_display_str(&txid))),
                ));

                let _ =
                    progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                if !confirmed_blocks_ids.is_empty() {
                    confirmed_blocks_ids.push_back(block_height);
                    sleep_ms(backoff_ms);
                    continue;
                }

                let tx_details_result = client.get_tx(&txid).await.map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "Failed to broadcast stacks transaction: {e}"
                    ))
                });

                let Ok(tx_details) = tx_details_result else {
                    // unable to fetch /v2/info
                    sleep_ms(backoff_ms);
                    continue;
                };
                let tx_result_bytes =
                    txtx_addon_kit::hex::decode(&tx_details.tx_result.hex[2..]).unwrap();

                match tx_details.tx_status {
                    TransactionStatus::Success => {
                        status_update.update_status(&ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Yellow,
                            &format!("Pending {}", progress_symbol[progress]),
                            &wrap_msg("Transaction included in block"),
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        result.outputs.insert(
                            "result".into(),
                            Value::buffer(tx_result_bytes, CLARITY_VALUE.clone()),
                        );
                    }
                    TransactionStatus::AbortByResponse => {
                        let diag = Diagnostic::error_from_string(format!(
                          "The transaction did not succeed because it was aborted during its execution: {}",
                          tx_details.tx_result.repr
                        ));
                        status_update.update_status(&ProgressBarStatus::new_err(
                            "Failed",
                            &wrap_msg("Transaction aborted"),
                            &diag,
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        return Err(diag);
                    }
                    TransactionStatus::AbortByPostCondition => {
                        let diag = Diagnostic::error_from_string(format!(
                        "This transaction would have succeeded, but was rolled back by a supplied post-condition: {}",
                        tx_details.tx_result.repr
                      ));
                        status_update.update_status(&ProgressBarStatus::new_err(
                            "Failed",
                            &wrap_msg("Transaction rolled back"),
                            &diag,
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                        return Err(diag);
                    }
                };
                confirmed_blocks_ids.push_back(node_info.stacks_tip_height);
            }

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
