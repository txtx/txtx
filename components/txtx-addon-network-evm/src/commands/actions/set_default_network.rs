use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionContext, CommandExecutionFutureResult,
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

use crate::constants::{CHAIN_ID, NETWORK_ID, RPC_API_URL};

lazy_static! {
    pub static ref SET_DEFAULT_NETWORK: PreCommandSpecification = {
        let mut command = define_command! {
            SetStacksGlobals => {
                name: "Configure EVM Network",
                matcher: "set_default_network",
                documentation: indoc!{r#"
                  The `evm::set_default_network` action can be used to set default network data for EVM Runbook inputs.
                  Any commands that have an input that matches the name of one of these default inputs will automatically have these default inputs applied.
                  This allows for more terse Runbooks, as some redundant data can be omitted.

                  For example, the `network_id` input is used in many EVM txtx commands. 
                  By setting this input once with `set_default_network`, the `network_id` can be omitted from subsequent EVM txtx commands.
                "#},
                implements_signing_capability: false,
                implements_background_task_capability: false,
                inputs: [
                    chain_id: {
                        documentation: indoc!{r#"A default EVM chain id to use."#},
                        typing: Type::uint(),
                        optional: false,
                        interpolable: true
                    },
                    network_id: {
                        documentation: indoc!{r#"A default EVM network id to use. Valid values are `"mainnet"`, `"testnet"` and `"devnet"`."#},
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    rpc_api_url: {
                        documentation: "A default EVM API RPC URL to use.",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    block_explorer_url: {
                        documentation: "A default block explorer URL used to verify contracts.",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [],
                example: txtx_addon_kit::indoc! {r#"
                // Coming soon
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

        let chain_id = args.get_expected_uint(CHAIN_ID)?;
        let network_id = args.get_expected_string(NETWORK_ID)?;
        let rpc_api_url = args.get_expected_string(RPC_API_URL)?;

        result
            .outputs
            .insert(CHAIN_ID.to_string(), Value::uint(chain_id));

        result
            .outputs
            .insert(NETWORK_ID.to_string(), Value::string(network_id.into()));

        result
            .outputs
            .insert(RPC_API_URL.to_string(), Value::string(rpc_api_url.into()));
        return_synchronous_ok(result)
    }
}
