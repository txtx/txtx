use std::vec;

use txtx_addon_kit::channel;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, StatusUpdater};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::codec::subgraph::{SubgraphPluginType, SubgraphRequest, SubgraphRequestClient};
use crate::constants::{PROGRAM_ID, RPC_API_URL, SUBGRAPH_REQUEST, SUBGRAPH_URL};
use crate::typing::{SvmValue, SUBGRAPH_EVENT};

lazy_static! {
    pub static ref DEPLOY_SUBGRAPH: PreCommandSpecification = {
        let mut command = define_command! {
            DeployProgram => {
                name: "Deploy SVM Program Subgraph",
                matcher: "deploy_subgraph",
                documentation: indoc!{r#"
                    `svm::deploy_subgraph` deploys allows specifying a schema for a subgraph for your program, 
                        which will automatically be registered and return an endpoint to see live chain data."#
                },
                implements_signing_capability: false,
                implements_background_task_capability: true,
                inputs: [
                    description: {
                        documentation: "A description of the subgraph.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    subgraph_name: {
                        documentation: "The name of the subgraph. This defaults to the command instance name.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    program_id: {
                        documentation: "The ID of the program to index as a subgraph.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    program_idl: {
                        documentation: "The IDL of the program, used to decode subgraph types.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    block_height: {
                        documentation: "The block height to start indexing from.",
                        typing: Type::integer(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    event: {
                        documentation: "A map of events to index in the subgraph.",
                        typing: SUBGRAPH_EVENT.clone(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    }
                ],
                outputs: [
                ],
                example: txtx_addon_kit::indoc! {r#"
                "#},
            }
        };

        command
    };
}

pub struct DeployProgram;
impl CommandImplementation for DeployProgram {
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
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::{constants::DESCRIPTION, types::commands::return_synchronous_ok};

        use crate::{
            codec::subgraph::{SubgraphEventDefinition, SubgraphRequest},
            constants::{BLOCK_HEIGHT, EVENT, PROGRAM_IDL, SUBGRAPH_NAME, SUBGRAPH_REQUEST},
        };
        let _rpc = values.get_expected_string(RPC_API_URL)?;
        let idl_str = values.get_expected_string(PROGRAM_IDL)?;
        let events =
            SubgraphEventDefinition::parse_map_values(values.get_expected_map(EVENT)?, idl_str)?;

        let block_height = values.get_expected_uint(BLOCK_HEIGHT)?;
        let program_id = SvmValue::to_pubkey(values.get_expected_value(PROGRAM_ID)?)
            .map_err(|e| diagnosed_error!("{e}"))?;

        let subgraph_name = values.get_string(SUBGRAPH_NAME).unwrap_or(&values.name);
        let description = values.get_string(DESCRIPTION).and_then(|s| Some(s.to_string()));

        let subgraph_request = SubgraphRequest::new(
            subgraph_name,
            description,
            &program_id,
            idl_str,
            events,
            block_height,
        )?;

        let mut result = CommandExecutionResult::new();
        result.insert(SUBGRAPH_REQUEST, subgraph_request.to_value()?);

        return_synchronous_ok(result)
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            let subgraph_request =
                SubgraphRequest::from_value(outputs.get_expected_value(SUBGRAPH_REQUEST)?)?;

            let rpc_api_url = inputs.get_expected_string(RPC_API_URL)?;

            let status_updater =
                StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

            let mut client = SubgraphRequestClient::new(
                rpc_api_url,
                subgraph_request,
                SubgraphPluginType::SurfpoolSubgraph,
                status_updater,
            );

            let url = client.deploy_subgraph().await?;

            result.outputs.insert(SUBGRAPH_URL.into(), Value::string(url));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
