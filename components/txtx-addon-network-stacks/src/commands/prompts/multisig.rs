use std::collections::HashMap;

use serde_json::Value as JsonValue;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::wallets::WalletSpecification;
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
    pub static ref MULTISIG: PreCommandSpecification = define_command! {
      Multisig => {
          name: "Multisig Stacks Transaction",
          matcher: "multisig",
          documentation: "The `multisig` prompt...",
          inputs: [
              public_key: {
                documentation: "The public keys of the expected signers.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
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
          // Coming soon
      "#},
      }
    };
}

pub struct Multisig;
impl CommandImplementation for Multisig {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
        _wallets: &HashMap<String, WalletSpecification>,
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
                "Key 'network_id' is missing"
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
