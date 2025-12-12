use clarity::util::sleep_ms;
use std::fmt::Write;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusUpdate,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::uuid::Uuid;

use crate::typing::StacksValue;

use crate::constants::{DEFAULT_CONFIRMATIONS_NUMBER, RPC_API_URL};
use crate::rpc::StacksRpc;

lazy_static! {
    pub static ref BROADCAST_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
        BroadcastStacksTransaction => {
            name: "Broadcast Stacks Transaction",
            matcher: "broadcast_transaction",
            documentation: "The `stacks::broadcast_transaction` action sends a signed transaction payload to the specified network.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                signed_transaction_bytes: {
                  documentation: "The signed transaction bytes that will be broadcasted to the network.",
                  typing: Type::buffer(),
                  optional: false,
                  tainting: true,
                  internal: false
                },
                network_id: {
                    documentation: indoc!{r#"The network id. Valid values are `"mainnet"`, `"testnet"` or `"devnet"`."#},
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues. The default is 1.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: false,
                    internal: false
                }
                // todo:
                // success_required: {
                //     documentation: "Success required.",
                //     typing: Type::bool(),
                //     optional: true,
                //     tainting: true,
                // internal: false
                // }
            ],
            outputs: [
                tx_id: {
                    documentation: "The transaction id.",
                    typing: Type::string()
                },
                value: {
                    documentation: "The transaction id.",
                    typing: Type::string()
                },
                result: {
                    documentation: "The transaction result.",
                    typing: Type::buffer()
                },
                decoded_result: {
                    documentation: "The transaction decoded result.",
                    typing: Type::string()
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
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
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
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
        _cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::{
            constants::SignerKey::SignedTransactionBytes, types::frontend::ProgressBarStatusColor,
        };

        use crate::{
            codec::cv::txid_display_str,
            constants::{
                DEFAULT_DEVNET_BACKOFF, DEFAULT_MAINNET_BACKOFF, NETWORK_ID, RPC_API_AUTH_TOKEN,
            },
            rpc::{RpcError, TransactionStatus},
        };

        let args = inputs.clone();
        let outputs = outputs.clone();

        let construct_did = construct_did.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let confirmations_required =
            args.get_expected_integer("confirmations")
                .unwrap_or(DEFAULT_CONFIRMATIONS_NUMBER as i128) as u64;

        let network_id = args.get_expected_string(NETWORK_ID)?.to_owned();

        let transaction_bytes = args.get_expected_buffer_bytes(SignerKey::SignedTransactionBytes)?;

        let rpc_api_url = args.get_expected_string(RPC_API_URL)?.to_owned();
        let rpc_api_auth_token =
            args.get_string(RPC_API_AUTH_TOKEN).and_then(|t| Some(t.to_owned()));

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
                    message: "Broadcasting Transaction".into(),
                    diagnostic: None,
                },
            );
            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let mut result = CommandExecutionResult::new();
            for (k, v) in outputs.iter() {
                result.outputs.insert(k.clone(), v.clone());
            }

            let mut s = String::from("0x");
            s.write_str(
                &transaction_bytes.clone().iter().map(|b| format!("{:02X}", b)).collect::<String>(),
            )
            .map_err(|e| {
                Diagnostic::error_from_string(format!("Failed to serialize transaction bytes: {e}"))
            })?;

            let client = StacksRpc::new(&rpc_api_url, &rpc_api_auth_token);

            let tx_result = loop {
                progress = (progress + 1) % progress_symbol.len();
                match client.post_transaction(&transaction_bytes).await {
                    Ok(res) => break res,
                    Err(RpcError::ContractAlreadyDeployed(data)) => {
                        result.outputs.insert("contract_id".into(), Value::string(data));
                        status_update.update_status(&ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Complete",
                            &format!("Contract deployed"),
                        ));
                        let _ =
                            progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update));
                        return Ok(result);
                    }
                    Err(e) => {
                        let diag =
                            diagnosed_error!("unable to broadcast Stacks transaction: {}", e);
                        status_update.update_status(&ProgressBarStatus::new_err(
                            "Failure",
                            &format!("Received RPC Error for Stacks Transaction"),
                            &diag,
                        ));
                        let _ = progress_tx
                            .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

                        return Err(diag);
                    }
                }
            };

            let txid = tx_result.txid;
            result.outputs.insert(format!("tx_id"), Value::string(txid.clone()));
            result.outputs.insert(format!("value"), Value::string(txid.clone()));

            let moved_txid = txid.clone();
            let moved_network_id = network_id.clone();
            let wrap_msg = move |msg: &str| {
                if is_supervised {
                    txtx_addon_kit::formatdoc! {
                    r#"<a target="_blank" href="https://explorer.hiro.so/txid/{}?chain={}&api={}">{}</a>"#,
                    moved_txid, moved_network_id, rpc_api_url, msg
                }
                .to_string()
                } else {
                    msg.to_string()
                }
            };

            let progress_tx = progress_tx.clone();
            let mut retry_count = 128;
            status_update.update_status(&ProgressBarStatus::new_msg(
                ProgressBarStatusColor::Yellow,
                &format!("Pending {}", progress_symbol[progress]),
                &wrap_msg(&format!("Transaction 0x{}", txid_display_str(&txid))),
            ));

            let _ = progress_tx.send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));

            let mut block_height = 0;
            let mut included_in_block = u64::MAX - confirmations_required;
            let backoff_ms = if network_id.eq("devnet") {
                DEFAULT_DEVNET_BACKOFF
            } else {
                DEFAULT_MAINNET_BACKOFF
            };

            loop {
                progress = (progress + 1) % progress_symbol.len();

                if block_height >= (included_in_block + confirmations_required - 1) {
                    status_update.update_status(&ProgressBarStatus::new_msg(
                        ProgressBarStatusColor::Green,
                        "Complete",
                        &wrap_msg(&format!(
                            "Confirmed {} {}",
                            &confirmations_required,
                            if confirmations_required.eq(&1) { "block" } else { "blocks" }
                        )),
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
                            "unable to broadcast Stacks transaction - {e}"
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

                let tx_details_result = client.get_tx(&txid).await.map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "unable to broadcast Stacks transaction - {e}"
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
                        if included_in_block != tx_details.block_height {
                            status_update.update_status(&ProgressBarStatus::new_msg(
                                ProgressBarStatusColor::Yellow,
                                &format!("Pending {}", progress_symbol[progress]),
                                &wrap_msg("Transaction included in block"),
                            ));
                            let _ = progress_tx
                                .send(BlockEvent::UpdateProgressBarStatus(status_update.clone()));
                            result.outputs.insert(
                                "result".into(),
                                StacksValue::generic_clarity_value(tx_result_bytes),
                            );
                            result.outputs.insert(
                                "decoded_result".into(),
                                Value::string(tx_details.tx_result.repr),
                            );
                            included_in_block = tx_details.block_height;
                        }
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
            }

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
