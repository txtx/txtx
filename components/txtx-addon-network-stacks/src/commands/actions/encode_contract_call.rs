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
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref ENCODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Stacks Contract Call",
          matcher: "call_contract",
          documentation: "The `call_contract` action encodes a valid contract call payload and serializes it as a hex string.",
          inputs: [
              contract_id: {
                  documentation: "The address and identifier of the contract to invoke.",
                  typing: Type::addon(CLARITY_PRINCIPAL.clone()),
                  optional: false,
                  interpolable: true
              },
              function_name: {
                  documentation: "The contract method to invoke.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              function_args: {
                  documentation: "The function arguments for the contract call.",
                  typing: Type::array(Type::addon(CLARITY_VALUE.clone())),
                  optional: true,
                  interpolable: true
              },
              network_id: {
                  documentation: "The network id used to validate the transaction version.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "The encoded contract call bytes.",
                  typing: Type::buffer()
              },
              network_id: {
                  documentation: "The network id of the encoded transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            action "my_ref" "stacks::call_contract" {
                description = "Encodes the contract call transaction."
                contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                function_name = "verify-and-update-price-feeds"
                function_args = [
                    encode_buffer(output.bitcoin_price_feed),
                    encode_tuple({
                        "pyth-storage-contract": encode_principal("${env.pyth_deployer}.pyth-store-v1"),
                        "pyth-decoder-contract": encode_principal("${env.pyth_deployer}.pyth-pnau-decoder-v1"),
                        "wormhole-core-contract": encode_principal("${env.pyth_deployer}.wormhole-core-v1")
                    })
                ]
            }
            output "bytes" {
              value = action.my_ref.bytes
            }
            output "network_id" {
              value = action.my_ref.network_id
            }
            // > bytes: 0x021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631
            // > network_id: testnet
        "#},
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
        defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        // Extract network_id
        let network_id = args
            .get("network_id")
            .and_then(|a| Some(a.expect_string()))
            .or(defaults.keys.get("network_id").map(|x| x.as_str()))
            .ok_or(Diagnostic::error_from_string(format!(
                "Key 'network_id' is missing"
            )))
            .unwrap()
            .to_string();

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

        // validate contract_id against network_id
        // todo, is there a better way to do this?
        let id_str = contract_id.to_string();
        let mainnet_match = id_str.starts_with("SP") && network_id.eq("mainnet");
        let testnet_match = id_str.starts_with("ST") && network_id.eq("testnet");
        if !mainnet_match && !testnet_match {
            unimplemented!(
                "contract id {} is not valid for network {}; return diagnostic",
                id_str,
                network_id
            );
        }

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
                        v => return diagnosed_error!("function argument is missing or wrong type {:?}", v),
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
        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

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
