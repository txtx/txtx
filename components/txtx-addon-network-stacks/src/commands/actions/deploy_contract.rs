use std::collections::HashMap;

use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        types::{Type, Value},
    },
    AddonDefaults,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: PreCommandSpecification = define_command! {
      StacksDeployContract => {
          name: "Stacks Contract Deployment",
          matcher: "deploy_contract",
          documentation: "Encode contract deployment payload",
          inputs: [
              clarity_value: {
                  documentation: "Any valid Clarity value",
                  typing: Type::bool(),
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
          example: txtx_addon_kit::indoc! {r#"
            action "my_ref" "stacks::deploy_contract" {
                description = "Encodes the contract call, prompts the user to sign, and broadcasts the set-token function."
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
        "#},
      }
    };
}
pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        unimplemented!()
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
