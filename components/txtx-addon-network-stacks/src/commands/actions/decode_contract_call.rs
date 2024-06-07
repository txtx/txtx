use clarity_repl::codec::{StacksTransaction, TransactionAuth, TransactionSpendingCondition};
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionContext, CommandExecutionFutureResult,
    CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::stacks_helpers::clarity_value_to_value;
use crate::typing::CLARITY_BUFFER;

lazy_static! {
    pub static ref DECODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Decode Stacks Contract Call",
          matcher: "decode_call_contract",
          documentation: "Coming soon",
          requires_signing_capability: false,
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
                typing: Type::uint()
              },
              signer: {
                documentation: "The transaction signer.",
                typing: Type::string()
              },
              fee: {
                documentation: "The transaction fee.",
                typing: Type::uint()
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    fn run_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();

        // Extract contract_id
        let buffer = args.get_expected_buffer("bytes", &CLARITY_BUFFER)?;

        match StacksTransaction::consensus_deserialize(&mut &buffer.bytes[..]) {
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
                    result
                        .outputs
                        .insert("nonce".to_string(), Value::uint(auth.nonce));
                    result.outputs.insert(
                        "signer".to_string(),
                        Value::string(format!("{}", auth.signer)),
                    );
                    result
                        .outputs
                        .insert("fee".to_string(), Value::uint(auth.tx_fee));
                } else {
                    unimplemented!("unimplemented auth decoding");
                }
            }
            Err(e) => unimplemented!("deserialize failed; return diagnostic: {}", e),
        };

        return_synchronous_result(Ok(result))
    }
}
