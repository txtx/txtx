use std::sync::Arc;
use std::vec;
use txtx_addon_kit::types::cloud_interface::{
    AuthenticatedCloudServiceRouter, CloudService, CloudServiceContext,
};

use serde_json::json;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, LogDispatcher};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_network_svm_types::subgraph::PluginConfig;
use txtx_addon_network_svm_types::{PDA_ACCOUNT_SUBGRAPH, TOKEN_ACCOUNT_SUBGRAPH};

use crate::constants::{
    DEVNET_SUBGRAPH_ENDPOINT, DO_INCLUDE_TOKEN, MAINNET_SUBGRAPH_ENDPOINT, NETWORK_ID, PROGRAM_ID,
    RPC_API_URL, SUBGRAPH_ENDPOINT_URL, SUBGRAPH_REQUEST, SUBGRAPH_URL,
};
use crate::typing::subgraph::{SubgraphPluginType, SubgraphRequest};
use crate::typing::{SvmValue, SUBGRAPH_EVENT};

lazy_static! {
    pub static ref DEPLOY_SUBGRAPH: PreCommandSpecification = {
        let mut command = define_command! {
            DeployProgram => {
                name: "Deploy SVM Program Subgraph",
                matcher: "deploy_subgraph",
                documentation: indoc!{r#"
                    `svm::deploy_subgraph` creates a live Graph QL database for your program.

                    This command takes a program ID to index, a block height to start indexing from, and a set of events to index.
                    This data is encoded as a request and sent to your surfnet (when deploying to localhost) or to the Surfpool cloud services (when deploying to devnet or mainnet).
                    When the request is received, the associated chain is indexed and the data is stored in a Graph QL database.
                "#},
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
                        documentation: "The name of the subgraph. This defaults to the event name.",
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
                        documentation: "The IDL of the program, used to decode the data to be indexed.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    slot: {
                        documentation: "The slot to start indexing from.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    block_height: {
                        documentation: "Deprecated. Use slot instead.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    event: {
                        documentation: "A map of events to index in the subgraph.",
                        typing: SUBGRAPH_EVENT.clone(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    pda: {
                        documentation: "The PDA account to index in the subgraph",
                        typing: PDA_ACCOUNT_SUBGRAPH.clone(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    token_account: {
                        documentation: "The token account to index in the subgraph",
                        typing: TOKEN_ACCOUNT_SUBGRAPH.clone(),
                        optional: true,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    }
                ],
                outputs: [
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "transfer_event_subgraph" "svm::deploy_subgraph" {
                        program_id = action.deploy.program_id
                        program_idl = action.deploy.program_idl
                        slot = action.deploy.slot
                        event {
                            name = "TransferEvent"
                        }
                    }
                    action "account_index" "svm::deploy_subgraph" {
                        program_id = action.deploy.program_id
                        program_idl = action.deploy.program_idl
                        slot = action.deploy.slot
                        pda {
                            type = "CustomAccount"
                            instruction {
                                name = "<instruction-using-this-account>"
                                account_name = "<name-of-account-in-instruction>"
                            }
                            instruction {
                                name = "<another-instruction-using-this-account>"
                                account_name = "<name-of-account-in-instruction>"
                            }
                        }
                    }
                "#},
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.implements_cloud_service = true;
        }
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
        _auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::{constants::DocumentationKey, types::commands::return_synchronous_ok};

        use crate::{
            constants::{
                BLOCK_HEIGHT, PROGRAM_IDL, SLOT, SUBGRAPH_ENDPOINT_URL, SUBGRAPH_NAME,
                SUBGRAPH_REQUEST,
            },
            typing::subgraph::SubgraphRequest,
        };

        let network_id = values.get_expected_string(NETWORK_ID)?;
        let (subgraph_url, do_include_token) = match network_id {
            "mainnet" | "mainnet-beta" => (MAINNET_SUBGRAPH_ENDPOINT, true),
            "devnet" => (DEVNET_SUBGRAPH_ENDPOINT, true),
            "localnet" | _ => (values.get_expected_string(RPC_API_URL)?, false),
        };

        let idl_str = values.get_expected_string(PROGRAM_IDL)?;

        let some_slot = values.get_uint(SLOT)?;
        let some_block_height = values.get_uint(BLOCK_HEIGHT)?;
        if some_slot.is_some() && some_block_height.is_some() {
            return Err(diagnosed_error!(
                "Both slot and block height are provided. The block height field is deprecated and should not be used. Use slot instead."
            ));
        }
        if some_slot.is_none() && some_block_height.is_none() {
            return Err(diagnosed_error!("Missing expected 'slot' value."));
        }
        let slot = some_slot.unwrap_or_else(|| some_block_height.unwrap());

        let program_id = SvmValue::to_pubkey(values.get_expected_value(PROGRAM_ID)?)
            .map_err(|e| diagnosed_error!("{e}"))?;

        let subgraph_name = values.get_string(SUBGRAPH_NAME).and_then(|s| Some(s.to_string()));
        let description = values.get_string(DocumentationKey::Description).and_then(|s| Some(s.to_string()));

        let subgraph_request = SubgraphRequest::parse_value_store_v0(
            subgraph_name,
            description,
            &program_id,
            idl_str,
            slot,
            construct_did,
            values,
        )?;

        let mut result = CommandExecutionResult::new();
        result.insert(SUBGRAPH_REQUEST, subgraph_request.to_value()?);
        result.insert(SUBGRAPH_ENDPOINT_URL, Value::string(subgraph_url.to_string()));
        result.insert(DO_INCLUDE_TOKEN, Value::bool(do_include_token));

        return_synchronous_ok(result)
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        _inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        _background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
        cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let construct_did = construct_did.clone();
        let cloud_service_context = cloud_service_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            let subgraph_request =
                SubgraphRequest::from_value(outputs.get_expected_value(SUBGRAPH_REQUEST)?)?;

            let subgraph_url = outputs.get_expected_string(SUBGRAPH_ENDPOINT_URL)?;
            let do_include_token = outputs.get_expected_bool(DO_INCLUDE_TOKEN)?;

            let logger =
                LogDispatcher::new(construct_did.as_uuid(), "svm::deploy_subgraph", &progress_tx);

            let mut client = SubgraphRequestClient::new(
                cloud_service_context
                    .expect("cloud service context not found")
                    .authenticated_cloud_service_router
                    .expect("authenticated cloud service router not found"),
                subgraph_request,
                SubgraphPluginType::SurfpoolSubgraph,
                logger,
                subgraph_url,
                do_include_token,
            );

            let url = client.deploy_subgraph().await?;

            result.outputs.insert(SUBGRAPH_URL.into(), Value::string(url));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub struct SubgraphRequestClient {
    router: Arc<dyn AuthenticatedCloudServiceRouter>,
    plugin_config: PluginConfig,
    logger: LogDispatcher,
    subgraph_endpoint_url: String,
    do_include_token: bool,
}

impl SubgraphRequestClient {
    pub fn new(
        router: Arc<dyn AuthenticatedCloudServiceRouter>,
        request: SubgraphRequest,
        plugin_name: SubgraphPluginType,
        logger: LogDispatcher,
        subgraph_endpoint_url: &str,
        do_include_token: bool,
    ) -> Self {
        Self {
            router,
            plugin_config: PluginConfig::new(plugin_name, request),
            logger,
            subgraph_endpoint_url: subgraph_endpoint_url.to_string(),
            do_include_token,
        }
    }

    pub async fn deploy_subgraph(&mut self) -> Result<String, Diagnostic> {
        let stringified_config = json![self.plugin_config.clone()];
        let params = serde_json::to_value(vec![stringified_config.to_string()])
            .map_err(|e| diagnosed_error!("could not serialize subgraph request: {e}"))?;

        let res = self
            .router
            .route(CloudService::svm_subgraph(
                &self.subgraph_endpoint_url,
                params,
                self.do_include_token,
            ))
            .await
            .map_err(|e| diagnosed_error!("failed to deploy subgraph: {e}"))?;

        self.logger.success_info(
            "Subgraph Deployed",
            format!(
                "Subgraph {} for program {} has been deployed",
                self.plugin_config.data.subgraph_name(),
                self.plugin_config.data.program_id(),
            ),
        );

        self.logger
            .success_info("Subgraph Deployed", format!("Your subgraph can be reached at {}", res));

        Ok(res)
    }
}
