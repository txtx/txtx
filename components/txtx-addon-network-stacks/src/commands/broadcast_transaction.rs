use clarity_repl::codec::TransactionVersion;
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::StacksTransaction};
use futures::FutureExt;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, fmt::Write, pin::Pin};
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementationAsync, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

lazy_static! {
    pub static ref BROADCAST_STACKS_TRANSACTION: CommandSpecification = define_async_command! {
        BroadcastStacksTransaction => {
            name: "Broadcast Stacks Transaction",
            matcher: "broadcast_transaction",
            documentation: "Broadcast a signed transaction payload",
            inputs: [
                description: {
                    documentation: "A description of the transaction being broadcasted.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                signed_transaction_bytes: {
                  documentation: "The signed transaction bytes that will be broadcasted to the network.",
                  typing: Type::buffer(),
                  optional: false,
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
        async move {
            let buffer_data = {
                let Some(bytes) = args.get("signed_transaction_bytes") else {
                    unimplemented!("return diagnostic");
                };
                match bytes {
                    Value::Primitive(PrimitiveValue::Buffer(bytes)) => bytes,
                    _ => unimplemented!(),
                }
            };
            let buffer_data = buffer_data.clone();
            let transaction =
                StacksTransaction::consensus_deserialize(&mut &buffer_data.bytes[..]).unwrap();
            let network = match transaction.version {
                TransactionVersion::Mainnet => "mainnet",
                TransactionVersion::Testnet => "testnet",
            };
            let url = format!("https://api.{}.hiro.so/v2/transactions", network);
            let mut s = String::from("0x");
            s.write_str(
                &buffer_data
                    .bytes
                    .clone()
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<String>(),
            )
            .unwrap();
            let client = reqwest::Client::new();
            let res = client
                .post(&url)
                .header("Content-Type", "application/octet-stream")
                .body(buffer_data.bytes)
                .send()
                .await
                .unwrap();

            match res.error_for_status_ref() {
                Ok(_) => {}
                Err(e) => return Err(Diagnostic::error_from_string(e.to_string())),
            };
            let tx_id = res.text().await.unwrap();

            result
                .outputs
                .insert(format!("tx_id"), Value::string(tx_id));

            Ok(result)
        }
        .boxed()
    }

    fn update_input_evaluation_results_from_user_input(
        ctx: &CommandSpecification,
        current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        input_name: String,
        value: String,
    ) {
        let (input_key, value) = match input_name.as_str() {
            "description" => {
                let description_input =
                    ctx.inputs.iter().find(|i| i.name == "description").expect(
                        "Send Stacks Transaction specification must have description input",
                    );

                let expected_type = description_input.typing.clone();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(Value::from_string(value, expected_type, None))
                };
                (description_input, value)
            }
            "web_interact" => {
                let mut object_values = HashMap::new();
                let web_interact_input =
                    ctx.inputs.iter().find(|i| i.name == "web_interact").expect(
                        "Send Stacks Transaction specification must have a web_interact input",
                    );
                let web_interact_input_object = web_interact_input
                    .as_object()
                    .expect("Send Stacks Transaction web interact input must be and object.");

                let value_json: JsonValue = match serde_json::from_str(&value) {
                    Ok(value) => value,
                    Err(_e) => unimplemented!(), // todo: return diagnostic
                };
                let value_json = value_json.as_object().unwrap();

                let transaction_hash_property = web_interact_input_object.iter().find(|p| p.name == "transaction_hash").expect("Send Stacks Transaction specification's web_interact input should have a transaction_hash property.");
                let tx_hash_expected_type = transaction_hash_property.typing.clone();
                let tx_hash_val = match value_json.get("transaction_hash") {
                    Some(value) => Some(PrimitiveValue::from_string(
                        value.to_string(),
                        tx_hash_expected_type,
                        None,
                    )),
                    None => None,
                };
                match tx_hash_val {
                    Some(value) => {
                        object_values.insert(transaction_hash_property.name.clone(), value);
                    }
                    None => {}
                };

                let nonce_property = web_interact_input_object.iter().find(|p| p.name == "nonce").expect("Send Stacks Transaction specification's web_interact input should have a nonce property.");
                let nonce_expected_type = nonce_property.typing.clone();
                let nonce_val = match value_json.get("nonce") {
                    Some(value) => Some(PrimitiveValue::from_string(
                        value.to_string(),
                        nonce_expected_type,
                        None,
                    )),
                    None => None,
                };
                match nonce_val {
                    Some(value) => {
                        object_values.insert(nonce_property.name.clone(), value);
                    }
                    None => {}
                };
                let result = Some(Ok(Value::Object(object_values)));
                (web_interact_input, result)
            }
            _ => unimplemented!("cannot parse serialized output for input {input_name}"),
        };
        match value {
            Some(value) => current_input_evaluation_result
                .inputs
                .insert(input_key.clone(), value),
            None => current_input_evaluation_result.inputs.remove(&input_key),
        };
    }
}
