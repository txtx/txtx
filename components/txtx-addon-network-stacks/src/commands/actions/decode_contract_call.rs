use clarity_repl::codec::{StacksTransaction, TransactionAuth, TransactionSpendingCondition};
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::CommandImplementation;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

lazy_static! {
    pub static ref DECODE_STACKS_CONTRACT_CALL: CommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Decode Stacks Contract Call",
          matcher: "decode_call_contract",
          documentation: "Decode transaction bytes.",
          inputs_parent_attribute: Some("use".to_string()),
          inputs: [
            transaction_bytes: {
                  documentation: "Transaction bytes to decode.",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
              }
          ],
          outputs: [
              contract_id: {
                documentation: "The decoded contract identifier.",
                typing: Type::string()
              },
              function_name: {
                documentation: "The decoded function name.",
                typing: Type::string()
              },
              args: {
                documentation: "The decoded function arguments.",
                typing: Type::string()
              },
              nonce: {
                documentation: "The decoded function arguments.",
                typing: Type::uint()
              },
              signer: {
                documentation: "The decoded function arguments.",
                typing: Type::string()
              },
              fee: {
                documentation: "The decoded function arguments.",
                typing: Type::uint()
              }
          ],
      }
    };
}

pub struct EncodeStacksContractCall;
impl CommandImplementation for EncodeStacksContractCall {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        // Extract contract_id
        let bytes = match args.get("transaction_bytes") {
            Some(Value::Primitive(PrimitiveValue::Buffer(buffer_data))) => &buffer_data.bytes,
            _ => todo!("transaction_bytes missing, return diagnostic"),
        };
        match StacksTransaction::consensus_deserialize(&mut &bytes[..]) {
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
                        // todo: set args output
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

        println!("==> {:?}", result);
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
}
