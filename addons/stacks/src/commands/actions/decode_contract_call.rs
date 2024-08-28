use crate::codec::codec::{
    StacksTransaction, TransactionAuth, TransactionPayload, TransactionSpendingCondition,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionFutureResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::constants::SIGNER;
use crate::stacks_helpers::clarity_value_to_value;

lazy_static! {
    pub static ref DECODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Decode Stacks Contract Call",
          matcher: "decode_call_contract",
          documentation: "Coming soon",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
            bytes: {
                  documentation: "The contract call transaction bytes to decode.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
              }
          ],
          outputs: [
              contract_id: {
                documentation: "The contract identifier.",
                typing: Type::string()
              },
              function_name: {
                documentation: "The function name.",
                typing: Type::string()
              },
              function_args: {
                documentation: "The function arguments.",
                typing: Type::string()
              },
              nonce: {
                documentation: "The transaction nonce.",
                typing: Type::integer()
              },
              signer: {
                documentation: "The transaction signer.",
                typing: Type::string()
              },
              fee: {
                documentation: "The transaction fee.",
                typing: Type::integer()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
         // Coming soon
      "#},
      }
    };
}

pub struct EncodeStacksContractCall;

impl CommandImplementation for EncodeStacksContractCall {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();

        // Extract contract_id
        let buffer = args.get_expected_buffer_bytes("bytes")?;

        match StacksTransaction::consensus_deserialize(&mut &buffer[..]) {
            Ok(tx) => {
                match tx.payload {
                    TransactionPayload::ContractCall(payload) => {
                        result.outputs.insert(
                            "contract_id".to_string(),
                            Value::string(format!("{}.{}", payload.address, payload.contract_name)),
                        );
                        result.outputs.insert(
                            "function_name".to_string(),
                            Value::string(format!("{}", payload.function_name)),
                        );
                        let function_args = payload
                            .function_args
                            .into_iter()
                            .map(|a| clarity_value_to_value(a))
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                        result
                            .outputs
                            .insert("function_args".to_string(), Value::array(function_args));
                    }
                    _ => unimplemented!("attempted to decode non-contract-call; return diagnostic"),
                }
                if let TransactionAuth::Standard(TransactionSpendingCondition::Singlesig(auth)) =
                    tx.auth
                {
                    result.outputs.insert("nonce".to_string(), Value::integer(auth.nonce as i128));
                    result
                        .outputs
                        .insert(SIGNER.to_string(), Value::string(format!("{}", auth.signer)));
                    result.outputs.insert("fee".to_string(), Value::integer(auth.tx_fee as i128));
                } else {
                    unimplemented!("unimplemented auth decoding");
                }
            }
            Err(e) => unimplemented!("deserialize failed; return diagnostic: {}", e),
        };

        return_synchronous_result(Ok(result))
    }
}
