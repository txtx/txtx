use crate::stacks_helpers::parse_clarity_value;
use crate::typing::{CLARITY_PRINCIPAL, CLARITY_VALUE, STACKS_CONTRACT_CALL};
use clarity::vm::types::PrincipalData;
use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::clarity::ClarityName;
use clarity_repl::codec::TransactionContractCall;
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

lazy_static! {
    pub static ref ENCODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Stacks Contract Call",
          matcher: "call_contract",
          documentation: "Encode contract call payload",
          inputs: [
              contract_id: {
                  documentation: "Address and identifier of the contract to invoke",
                  typing: Type::addon(CLARITY_PRINCIPAL.clone()),
                  optional: false,
                  interpolable: true
              },
              function_name: {
                  documentation: "Method to invoke",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              function_args: {
                  documentation: "Args to provide",
                  typing: Type::array(Type::addon(CLARITY_VALUE.clone())),
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

        // Extract contract_address
        let contract_id = match args.get("contract_id") {
            Some(Value::Primitive(PrimitiveValue::Buffer(contract_id))) => {
                match parse_clarity_value(&contract_id.bytes, &contract_id.typing) {
                    Ok(cv) => match cv {
                        clarity::vm::Value::Principal(PrincipalData::Contract(c)) => c,
                        cv => todo!("unexpected clarity value {cv}"),
                    },
                    Err(e) => return Err(e),
                }
            }
            Some(Value::Primitive(PrimitiveValue::String(contract_id))) => {
                clarity::vm::types::QualifiedContractIdentifier::parse(contract_id).unwrap()
            }
            _ => todo!("contract_id is missing or wrong type, return diagnostic"),
        };
        // Extract derivation path
        let function_name = match args.get("function_name") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => value.clone(),
            _ => todo!("function_name missing or wrong type, return diagnostic"),
        };

        let function_args = match args.get("function_args") {
            Some(Value::Array(args)) => {
                let mut function_args = vec![];
                for arg in args.iter() {
                    let function_arg = match arg {
                        // todo maybe we can assume some types?
                        Value::Primitive(PrimitiveValue::Buffer(buffer_data)) => {
                            match parse_clarity_value(&buffer_data.bytes, &buffer_data.typing) {
                                Ok(v) => v,
                                Err(e) => return Err(e),
                            }
                        }
                        v => todo!("function argument is missing or wrong type {:?}", v), // return diag
                    };

                    function_args.push(function_arg)
                }
                function_args
            }
            _ => todo!("function_args missing, return diagnostic"),
        };

        let payload = TransactionPayload::ContractCall(TransactionContractCall {
            contract_name: contract_id.name.clone(),
            address: StacksAddress::from(contract_id.issuer.clone()),
            function_name: ClarityName::try_from(function_name).unwrap(),
            function_args,
        });

        let mut bytes = vec![];
        payload.consensus_serialize(&mut bytes).unwrap();
        let value = Value::buffer(bytes, STACKS_CONTRACT_CALL.clone());

        result.outputs.insert("bytes".to_string(), value);

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
