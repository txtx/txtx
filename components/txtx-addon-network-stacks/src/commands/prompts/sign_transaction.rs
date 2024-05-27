use std::collections::HashMap;

use serde_json::Value as JsonValue;
use txtx_addon_kit::types::commands::{CommandInstance, PreCommandSpecification};
use txtx_addon_kit::types::frontend::ActionItem;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

use crate::typing::STACKS_SIGNED_TRANSACTION;

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
      SignStacksTransaction => {
          name: "Sign Stacks Transaction",
          matcher: "sign_transaction",
          documentation: "The `sign_transaction` prompt signs an encoded transaction payload in the txtx web ui with an in-browser wallet.",
          inputs: [
              transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
              },
              signed_transaction_bytes: {
                  documentation: indoc!{r#"
                    The signed transaction bytes. 
                    In most cases, this input should not be set. 
                    Setting this input skips the Runbook step to sign in the browser.
                  "#},
                  typing: Type::buffer(),
                  optional: true,
                  interpolable: true
              },
              nonce: {
                  documentation: "The transaction nonce.",
                  typing: Type::uint(),
                  optional: true,
                  interpolable: true
              },
              network_id: {
                  documentation: indoc!{r#"The network id, which is used to set the transaction version. Can be `"testnet"` or `"mainnet"`."#},
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              network_id: {
                  documentation: "Network id of the signed transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          prompt "my_ref" "stacks::sign_transaction" {
              transaction_payload_bytes = encode_buffer("0x021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631")
              network_id = "testnet"
          }
          output "signed_bytes" {
            value = prompt.my_ref.signed_transaction_bytes
          }
          // > signed_bytes: 0x8080000000040063A5EDA39412C016478AE5A8C300843879F78245000000000000000100000000000004B0000182C1712C31B7F683F6C56EEE8920892F735FC0118C98FD10C1FDAA85ABEC2808063773E5F61229D76B29784B8BBBBAAEA72EEA701C92A4FE15EF3B9E32A373D8020100000000021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631
      "#},
      }
    };
}

pub struct SignStacksTransaction;
impl CommandImplementation for SignStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }
    fn get_action(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _uuid: &ConstructUuid,
        _index: u16,
        _instance: &CommandInstance,
    ) -> Option<ActionItem> {
        todo!()
    }
    fn run(
        ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        match args.get("signed_transaction_bytes") {
            Some(val) => {
                result
                    .outputs
                    .insert("signed_transaction_bytes".to_string(), val.clone());
            }
            None => {}
        };

        let network_id = args
            .get("network_id")
            .and_then(|a| Some(a.expect_string()))
            .or(defaults.keys.get("network_id").map(|x| x.as_str()))
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': attribute 'network_id' is missing",
                ctx.matcher
            )))
            .unwrap()
            .to_string();

        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        ctx: &CommandSpecification,
        current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String, // todo: this may be needed to see which input is being edited (if only one at a time)
        value: String,
    ) {
        let value_json: JsonValue = match serde_json::from_str(&value) {
            Ok(value) => value,
            Err(_e) => unimplemented!(), // todo: return diagnostic
        };
        let value_json = value_json.as_object().unwrap(); // todo

        let transaction_signature_property = ctx.inputs.iter().find(|p| p.name == "signed_transaction_bytes").expect("Sign Stacks Transaction specification's web_interact input should have a signed_transaction_bytes property.");
        let expected_type = transaction_signature_property.typing.clone();
        match value_json.get("signed_transaction_bytes") {
            Some(value) => {
                current_input_evaluation_result.inputs.insert(
                    transaction_signature_property.clone(),
                    Value::from_string(
                        value.as_str().unwrap().to_string(),
                        expected_type,
                        Some(STACKS_SIGNED_TRANSACTION.clone()),
                    ),
                );
            }
            None => {
                current_input_evaluation_result
                    .inputs
                    .remove(&transaction_signature_property);
            }
        };

        let nonce_property = ctx.inputs.iter().find(|p| p.name == "nonce").expect("Send Stacks Transaction specification's web_interact input should have a nonce property.");
        let nonce_expected_type = nonce_property.typing.clone();
        match value_json.get("nonce") {
            Some(value) => {
                current_input_evaluation_result.inputs.insert(
                    nonce_property.clone(),
                    Value::from_string(value.to_string(), nonce_expected_type, None),
                );
            }
            None => {
                current_input_evaluation_result
                    .inputs
                    .remove(&nonce_property);
            }
        };
    }
}
