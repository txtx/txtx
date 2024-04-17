use clarity_repl::codec::{StacksTransaction, TransactionAuth, TransactionSpendingCondition};
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{CommandImplementation, PreCommandSpecification};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

use crate::stacks_helpers::clarity_value_to_value;

lazy_static! {
    pub static ref DECODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Decode Stacks Contract Call",
          matcher: "decode_call_contract",
          documentation: "Decode transaction bytes.",
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
        let bytes = match args.get("bytes") {
            Some(Value::Primitive(PrimitiveValue::Buffer(buffer_data))) => &buffer_data.bytes,
            _ => todo!("bytes missing, return diagnostic"),
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
                        let function_args = payload
                            .function_args
                            .into_iter()
                            .map(|a| clarity_value_to_value(a))
                            .collect::<Result<Vec<_>, _>>()?;
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
