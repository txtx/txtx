use serde_json::Value as JsonValue;
use std::collections::HashMap;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveType, PrimitiveValue, Type, Value},
};

lazy_static! {
  pub static ref SEND_STACKS_TRANSACTION: CommandSpecification = define_command! {
      SendStacksTransaction => {
          name: "Send Stacks Transaction",
          matcher: "send_transaction",
          documentation: "Send an encoded transaction payload",
          inputs: [
              description: {
                  documentation: "A description of the transaction being sent.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              },
              no_interact: {
                  documentation: "Any valid Clarity value",
                  typing: define_object_type! [], // todo
                  optional: true,
                  interpolable: true
              },
              cli_interact: {
                  documentation: "Any valid Clarity value",
                  typing: define_object_type! [], // todo
                  optional: true,
                  interpolable: true
              },
              web_interact: {
                  documentation: "Some documentation", // todo
                  typing: define_object_type! [
                    encoded_bytes: {
                        documentation: "The encoded transaction bytes to be sent.",
                        typing: PrimitiveType::UnsignedInteger,
                        optional: false,
                        interpolable: true
                    },
                    transaction_hash: {
                        documentation: "The transaction hash.",
                        typing: PrimitiveType::String,
                        optional: true,
                        interpolable: true
                    },
                    nonce: {
                        documentation: "The nonce of the address sending the transaction.",
                        typing: PrimitiveType::UnsignedInteger,
                        optional: true,
                        interpolable: true
                    }
                  ],
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
            transaction_hash: {
                  documentation: "The transaction hash",
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
pub struct SendStacksTransaction;
impl CommandImplementation for SendStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        unimplemented!()
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
