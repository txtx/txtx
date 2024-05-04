use std::{collections::HashMap, fmt::Write, pin::Pin};
use txtx_addon_kit::reqwest::{self, StatusCode};
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementationAsync, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};
use txtx_addon_kit::AddonDefaults;

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
                network_id: {
                  documentation: "The id of the network to broadcast to.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                }
            ],
            outputs: [
              tx_id: {
                    documentation: "The transaction id.",
                    typing: Type::string()
                },
                nonce: {
                      documentation: "The nonce of the address sending the transaction.",
                      typing: Type::uint()
                }
            ],
        }
    };
}
pub struct BroadcastStacksTransaction;
impl CommandImplementationAsync for BroadcastStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<CommandExecutionResult, Diagnostic>>>> //todo: alias type
    {
        let mut result = CommandExecutionResult::new();
        let args = args.clone();
        let future = async move {
            let buffer_data = {
                let Some(bytes) = args.get("signed_transaction_bytes") else {
                    unimplemented!("return diagnostic");
                };
                match bytes {
                    Value::Primitive(PrimitiveValue::Buffer(bytes)) => bytes.clone(),
                    _ => unimplemented!(),
                }
            };
            let transaction = StacksTransaction::consensus_deserialize(&mut &buffer_data.bytes[..])
                .map_err(|e| {
                    Diagnostic::error_from_string(format!("Failed to deserialize transaction: {e}"))
                })?;
            let network = match transaction.version {
                TransactionVersion::Mainnet => "mainnet".to_string(),
                TransactionVersion::Testnet => "testnet".to_string(),
            };

            let network_id = match args.get("network_id") {
                Some(network_id) => {
                    match network_id {
                        Value::Primitive(PrimitiveValue::String(network_id)) => {
                            if network_id.eq(&network) {
                                Ok(network_id)
                            } else {
                                Err(Diagnostic::error_from_string(format!("Specified network id, {}, does not match transaction version, {}.", network_id, network)))
                            }
                        }
                        invalid => Err(Diagnostic::error_from_string(format!(
                            "Network id must be a string, received {:?}.",
                            invalid
                        ))),
                    }
                }
                None => Ok(&network),
            }?;

            let url = format!("https://api.{}.hiro.so/v2/transactions", network_id);

            let mut s = String::from("0x");
            s.write_str(
                &buffer_data
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
                .post(&url)
                .header("Content-Type", "application/octet-stream")
                .body(buffer_data.bytes)
                .send()
                .await
                .map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "Failed to broadcast stacks transaction: {e}"
                    ))
                })?;

            let status = res.status();
            let result_text = res.text().await.map_err(|e| {
                Diagnostic::error_from_string(format!(
                    "Failed to parse broadcasted Stacks transaction result: {e}"
                ))
            })?;

            match status {
                StatusCode::OK => {
                    result
                        .outputs
                        .insert(format!("tx_id"), Value::string(result_text));
                    Ok(())
                }
                _ => Err(Diagnostic::error_from_string(result_text)),
            }?;

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
