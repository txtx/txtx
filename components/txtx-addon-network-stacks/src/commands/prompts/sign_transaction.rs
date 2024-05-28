use std::collections::HashMap;

use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionContext, CommandExecutionFutureResult,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

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
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(), ActionItemRequest> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        spec: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
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
                spec.matcher
            )))
            .unwrap()
            .to_string();

        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

        return_synchronous_ok(result)
    }
}
