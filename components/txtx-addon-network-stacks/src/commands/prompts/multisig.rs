use std::collections::HashMap;

use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionContext, CommandExecutionFutureResult,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::wallets::WalletSpecification;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

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
        _wallets: &HashMap<String, WalletSpecification>,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(), ActionItemRequest> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
        _wallets: &HashMap<String, WalletSpecification>,
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
                "Key 'network_id' is missing"
            )))
            .unwrap()
            .to_string();

        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

        return_synchronous_ok(result)
    }
}
