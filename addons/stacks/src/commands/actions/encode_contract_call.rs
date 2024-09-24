use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionFutureResult, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::typing::{STACKS_CV_GENERIC, STACKS_CV_PRINCIPAL};

use super::encode_contract_call;

lazy_static! {
    pub static ref ENCODE_STACKS_CONTRACT_CALL: PreCommandSpecification = define_command! {
        EncodeStacksContractCall => {
          name: "Encode a Stacks Contract Call",
          matcher: "encode_contract_call",
          documentation: "The `stacks::call_contract` action encodes a valid contract call payload and serializes it as a hex string.",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
              contract_id: {
                  documentation: "The address and identifier of the contract to invoke.",
                  typing: Type::addon(STACKS_CV_PRINCIPAL),
                  optional: false,
                  tainting: true,
                  internal: false
              },
              function_name: {
                  documentation: "The contract method to invoke.",
                  typing: Type::string(),
                  optional: false,
                  tainting: true,
                  internal: false
              },
              function_args: {
                  documentation: "The function arguments for the contract call.",
                  typing: Type::array(Type::addon(STACKS_CV_GENERIC)),
                  optional: true,
                  tainting: true,
                  internal: false
              },
              network_id: {
                documentation: indoc!{r#"The network id. Can be `"mainnet"`, `"testnet"` or `"devnet"`."#},
                  typing: Type::string(),
                  optional: false,
                  tainting: true,
                  internal: false
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
                action "my_ref" "stacks::encode_contract_call" {
                    description = "Encodes the contract call transaction."
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                    function_name = "verify-and-update-price-feeds"
                    function_args = [
                        stacks::cv_buff(variable.bitcoin_price_feed),
                        stacks::cv_tuple({
                            "pyth-storage-contract": stacks::cv_principal("${input.pyth_deployer}.pyth-store-v1"),
                            "pyth-decoder-contract": stacks::cv_principal("${input.pyth_deployer}.pyth-pnau-decoder-v1"),
                            "wormhole-core-contract": stacks::cv_principal("${input.pyth_deployer}.wormhole-core-v1")
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
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();

        // Extract network_id
        let network_id = args.get_expected_string("network_id")?.to_owned();
        let contract_id_value = args.get_expected_value("contract_id")?;
        let function_name = args.get_expected_string("function_name")?;
        let empty_vec = vec![];
        let function_args_values = args.get_expected_array("function_args").unwrap_or(&empty_vec);

        let bytes = encode_contract_call(
            spec,
            function_name,
            function_args_values,
            &network_id,
            contract_id_value,
        )?;

        result.outputs.insert("bytes".to_string(), bytes);
        result.outputs.insert("network_id".to_string(), Value::string(network_id));
        return_synchronous_ok(result)
    }
}
