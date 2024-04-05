use crate::typing::STACKS_CONTRACT_CALL;
use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::ClarityName;
use clarity_repl::codec::TransactionContractCall;
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

lazy_static! {
    pub static ref ENCODE_STACKS_CONTRACT_CALL: CommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Stacks Contract Call",
          matcher: "call_contract",
          documentation: "Encode contract call payload",
          inputs: [
              description: {
                  documentation: "Description of the variable",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              },
              contract_id: {
                  documentation: "Contract identifier to invoke",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              function_name: {
                  documentation: "Method to invoke",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              args: {
                  documentation: "Args to provide",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "Encoded contract call",
                  typing: Type::buffer()
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
        let contract_id = match args.get("contract_id") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => {
                match QualifiedContractIdentifier::parse(value) {
                    Ok(value) => value,
                    _ => todo!("contract_id invalid, return diagnostic"),
                }
            }
            _ => todo!("contract_id missing, return diagnostic"),
        };
        // Extract derivation path
        let function_name = match args.get("function_name") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => value.clone(),
            _ => todo!("function_name missing, return diagnostic"),
        };

        let payload = TransactionPayload::ContractCall(TransactionContractCall {
            contract_name: contract_id.name.clone(),
            address: StacksAddress::from(contract_id.issuer.clone()),
            function_name: ClarityName::try_from(function_name).unwrap(),
            function_args: vec![],
        });

        let mut bytes = vec![];
        payload.consensus_serialize(&mut bytes).unwrap();
        let value = Value::buffer(bytes, STACKS_CONTRACT_CALL.clone());

        result.outputs.insert("bytes".to_string(), value);

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
