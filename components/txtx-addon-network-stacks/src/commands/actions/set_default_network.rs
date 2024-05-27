use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandImplementation, CommandInstance, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItem;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref SET_DEFAULT_NETWORK: PreCommandSpecification = {
        let mut command = define_command! {
            SetStacksGlobals => {
                name: "Configure Stacks Network",
                matcher: "set_default_network",
                documentation: indoc!{r#"
                  The `set_default_network` action can be used to set default network data for Stacks Runbook inputs.
                  Any commands that have an input that matches the name of one of these default inputs will automatically have these default inputs applied.
                  This allows for more terse Runbooks, as some redundant data can be omitted.

                  For example, the `network_id` input is used in many Stacks txtx commands. 
                  By setting this input once with `set_default_network`, the `network_id` can be omitted from subsequent Stacks txtx commands.
                "#},
                inputs: [
                    network_id: {
                        documentation: indoc!{r#"A default Stacks network id to use. Valid values are `"mainnet"` and `"testnet"`."#},
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    stacks_api_url: {
                        documentation: "A default Stacks API RPC URL to use.",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    }
                ],
                outputs: [],
                example: txtx_addon_kit::indoc! {r#"
                action "my_ref" "stacks::set_default_network" {
                    description = "Sets the default network id and Stacks API url."
                    network_id = "mainnet"
                    stacks_api_url = "https://api.mainnet.hiro.so"
                }
                prompt "signed_bytes" "stacks::sign_transaction" {
                  transaction_payload_bytes = encode_buffer("0x021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631")
                  // network_id = "testnet" // note, the network_id can be omitted
                }
                output "signed_bytes" {
                  value = prompt.my_ref.signed_transaction_bytes
                }
                // > signed_bytes: 0x8080000000040063A5EDA39412C016478AE5A8C300843879F78245000000000000000100000000000004B0000182C1712C31B7F683F6C56EEE8920892F735FC0118C98FD10C1FDAA85ABEC2808063773E5F61229D76B29784B8BBBBAAEA72EEA701C92A4FE15EF3B9E32A373D8020100000000021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631
            "#},
                }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.update_addon_defaults = true;
        }
        command
    };
}

pub struct SetStacksGlobals;
impl CommandImplementation for SetStacksGlobals {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }
    fn get_action(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _uuid: &ConstructUuid,
        _index: u16,
        _instance: &CommandInstance,
    ) -> Option<ActionItem> {
        None
    }
    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        let stacks_network = args.get("network_id").unwrap().expect_string();
        let stacks_api_url = args.get("stacks_api_url").unwrap().expect_string();

        result.outputs.insert(
            "network_id".to_string(),
            Value::string(stacks_network.into()),
        );

        result.outputs.insert(
            "stacks_api_url".to_string(),
            Value::string(stacks_api_url.into()),
        );
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
