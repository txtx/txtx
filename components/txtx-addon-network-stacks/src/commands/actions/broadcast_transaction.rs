use clarity::util::sleep_ms;
use std::collections::VecDeque;
use std::{collections::HashMap, fmt::Write, pin::Pin};
use txtx_addon_kit::reqwest;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementationAsync, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;
use serde_json::Value as JsonValue;

use crate::typing::CLARITY_VALUE;

lazy_static! {
    pub static ref BROADCAST_STACKS_TRANSACTION: PreCommandSpecification = define_async_command! {
        BroadcastStacksTransaction => {
            name: "Broadcast Stacks Transaction",
            matcher: "broadcast_transaction",
            documentation: "Broadcast a signed transaction payload",
            inputs: [
                signed_transaction_bytes: {
                  documentation: "The signed transaction bytes that will be broadcasted to the network.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
                },
                stacks_api_url: {
                  documentation: "The URL of the Stacks API to broadcast to.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                },
                confirmations: {
                    documentation: "The number of blocks required.",
                    typing: Type::uint(),
                    optional: true,
                    interpolable: true
                },
                success_required: {
                    documentation: "Success required.",
                    typing: Type::bool(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
              tx_id: {
                    documentation: "The transaction id.",
                    typing: Type::string()
            },
                result: {
                    documentation: "The result of the transaction",
                    typing: Type::buffer()
                }
            ],
        }
    };
}
pub struct BroadcastStacksTransaction;
impl CommandImplementationAsync for BroadcastStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
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

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<CommandExecutionResult, Diagnostic>>>> //todo: alias type
    {
        let mut result = CommandExecutionResult::new();
        let args = args.clone();

        let transaction_bytes = args
            .get("signed_transaction_bytes")
            .unwrap()
            .expect_buffer_data()
            .clone();

        let confirmations_required = args
            .get("confirmations")
            .unwrap_or(&Value::uint(3))
            .expect_uint()
            .clone() as usize;

        let api_url = args
            .get("stacks_api_url")
            .and_then(|a| Some(a.expect_string()))
            .or(defaults.keys.get("stacks_api_url").map(|x| x.as_str()))
            .ok_or(Diagnostic::error_from_string(format!(
                "Key 'stacks_api_url' is missing"
            )))
            .unwrap()
            .to_string();

        let future = async move {
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

            let client = reqwest::Client::new();
            let res = client
                .post(format!("{}/v2/transactions", api_url))
                .header("Content-Type", "application/octet-stream")
                .body(transaction_bytes.bytes)
                .send()
                .await
                .map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "Failed to broadcast stacks transaction: {e}"
                    ))
                })?;

            let status = res.status();
            if !status.is_success() {
                let transaction: PostTransactionResponseError = res.json().await.map_err(|e| {
                    println!("{:?}", e.to_string());
                    Diagnostic::error_from_string(format!(
                        "Failed to parse broadcasted Stacks transaction result: {e}"
                    ))
                })?;
                return Err(Diagnostic::error_from_string(format!("{:?}", transaction.reason)));
            }
            let mut txid = res.text().await.map_err(|e| {
                println!("{:?}", e.to_string());
                Diagnostic::error_from_string(format!(
                    "Failed to parse broadcasted Stacks transaction result: {e}"
                ))
            })?;

            // Strip extra double quotes
            txid = txid[1..65].to_string();

            result
                .outputs
                .insert(format!("tx_id"), Value::string(txid.clone()));

            let mut block_height = 0;
            let mut confirmed_blocks_ids = VecDeque::new();
            let backoff_ms = 5000;
            loop {
                println!("{:?}", confirmed_blocks_ids);

                if confirmed_blocks_ids.len() >= confirmations_required {
                    break;
                }

                let node_info_response = client
                    .get(format!("{}/v2/info", api_url))
                    .send()
                    .await
                    .map_err(|e| {
                        Diagnostic::error_from_string(format!(
                            "Failed to broadcast stacks transaction: {e}"
                        ))
                    });

                let Ok(encoded_node_info) = node_info_response else {
                    // unable to fetch /v2/info
                    sleep_ms(backoff_ms);
                    continue;
                };

                if !encoded_node_info.status().is_success() {
                    // unable to fetch /extended/v1/tx
                    sleep_ms(backoff_ms);
                    continue;
                }

                let decoded_node_info: Result<GetNodeInfoResponse, _> =
                    encoded_node_info.json().await;

                let Ok(node_info) = decoded_node_info else {
                    // unable to fetch /v2/info
                    sleep_ms(backoff_ms);
                    continue;
                };

                if node_info.stacks_tip_height == block_height {
                    // no new block
                    sleep_ms(backoff_ms);
                    continue;
                }

                block_height = node_info.stacks_tip_height;

                if !confirmed_blocks_ids.is_empty() {
                    confirmed_blocks_ids.push_back(block_height);
                    sleep_ms(backoff_ms);
                    continue;
                }

                let tx_encoded_response_res = client
                    .get(format!(
                        "{}/extended/v1/tx/{}",
                        api_url, txid
                    ))
                    .send()
                    .await
                    .map_err(|e| {
                        Diagnostic::error_from_string(format!(
                            "Failed to broadcast stacks transaction: {e}"
                        ))
                    });

                let Ok(tx_encoded_response) = tx_encoded_response_res else {
                    // unable to fetch /v2/info
                    sleep_ms(backoff_ms);
                    continue;
                };

                if !tx_encoded_response.status().is_success() {
                    // unable to fetch /extended/v1/tx
                    sleep_ms(backoff_ms);
                    continue;
                }

                let tx_decoded_res: Result<GetTransactionResponse, _> =
                    tx_encoded_response.json().await;
                let Ok(tx_decoded) = tx_decoded_res else {
                    // unable to decode
                    sleep_ms(backoff_ms);
                    continue;
                };

                let tx_result_bytes =
                    txtx_addon_kit::hex::decode(&tx_decoded.tx_result.hex[2..]).unwrap();
                result.outputs.insert(
                    "result".into(),
                    Value::buffer(tx_result_bytes, CLARITY_VALUE.clone()),
                );
                confirmed_blocks_ids.push_back(node_info.stacks_tip_height);
            }

            println!("Done! {:?}", confirmed_blocks_ids);

            Ok(result)
        };

        Box::pin(future)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        unimplemented!()
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetNodeInfoResponse {
    pub burn_block_height: u64,
    pub stable_burn_block_height: u64,
    pub server_version: String,
    pub network_id: u32,
    pub parent_network_id: u32,
    pub stacks_tip_height: u64,
    pub stacks_tip: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PostTransactionResponseError {
    pub txid: String,
    pub error: Option<String>,
    pub reason: Option<String>,
    pub reason_data: Option<JsonValue>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetTransactionResponse {
    pub tx_id: String,
    pub tx_status: String,
    pub tx_result: GetTransactionResult,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetTransactionResult {
    pub hex: String,
    pub repr: String,
}
