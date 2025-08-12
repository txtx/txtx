use std::thread::sleep;
use std::time::Duration;

use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, StatusUpdater};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::{types::RunbookSupervisionContext, ConstructDid};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::docker::RollupDeployer;
use crate::codec::rollup_config::RollupConfig;
use crate::constants::{
    BATCHER_SECRET_KEY, JWT, L1_DEPLOYMENT_ADDRESSES, L1_RPC_API_URL, L1_RPC_KIND,
    PROPOSER_SECRET_KEY, ROLLUP_CONFIG, ROLLUP_CONTAINER_IDS, SEQUENCER_SECRET_KEY, WORKING_DIR,
};
use crate::typing::{ROLLUP_CONFIG_TYPE, ROLLUP_CONTAINER_IDS_TYPE};

lazy_static! {
    pub static ref SETUP_ROLLUP: PreCommandSpecification = define_command! {
        SetupRollup => {
            name: "Coming Soon",
            matcher: "setup_rollup",
            documentation: "The `ovm::setup_rollup` action takes some L2 settings and deployment addresses of the L1 contracts and generates the `genesis.json` and `rollup.json` files needed to start a L2 OP node.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                l1_rpc_api_url: {
                    documentation: "The URL of the L1 EVM API used to fetch data.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                l1_rpc_kind: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                working_dir: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                sequencer_secret_key: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                batcher_secret_key: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                proposer_secret_key: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                jwt: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                rollup_config: {
                    documentation: "Coming soon.",
                    typing: ROLLUP_CONFIG_TYPE.clone(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                l1_deployment_addresses: {
                    documentation: "Coming soon.",
                    typing: Type::arbitrary_object(),
                    optional: false,
                    tainting: false,
                    internal: false
                }
            ],
            outputs: [
                rollup_container_ids: {
                    documentation: "Coming soon.",
                    typing: ROLLUP_CONTAINER_IDS_TYPE.clone()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                // Coming soon
            "#},
        }
    };
}

pub struct SetupRollup;
impl CommandImplementation for SetupRollup {
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
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        _outputs: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
        _cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let inputs = inputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let future = async move {
            let network_name = "ovm_network";
            let working_dir = inputs.get_expected_string(WORKING_DIR)?;
            let l1_rpc_api_url = inputs.get_expected_string(L1_RPC_API_URL)?;
            let l1_rpc_kind = inputs.get_string(L1_RPC_KIND);
            let sequencer_secret_key = inputs.get_expected_string(SEQUENCER_SECRET_KEY)?;
            let batcher_secret_key = inputs.get_expected_string(BATCHER_SECRET_KEY)?;
            let proposer_secret_key = inputs.get_expected_string(PROPOSER_SECRET_KEY)?;
            let jwt = inputs.get_expected_string(JWT)?;

            let rollup_config =
                RollupConfig::new(inputs.get_expected_map(ROLLUP_CONFIG)?, l1_rpc_api_url).await?;

            let l1_deployment_addresses = inputs.get_expected_object(L1_DEPLOYMENT_ADDRESSES)?;

            let mut rollup_deployer = RollupDeployer::new(
                network_name,
                working_dir,
                l1_rpc_api_url,
                l1_rpc_kind,
                &rollup_config,
                &l1_deployment_addresses,
                sequencer_secret_key,
                batcher_secret_key,
                proposer_secret_key,
                jwt,
            )?;
            let mut status_updater =
                StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

            status_updater.propagate_pending_status("Initializing rollup configuration");

            rollup_deployer.init().await.map_err(|e| {
                let diag = diagnosed_error!("Failed to initialize rollup: {e}");
                status_updater.propagate_failed_status("Failed to initialize rollup", &diag);
                diag
            })?;

            status_updater.propagate_success_status("Initialized", "Rollup configuration complete");

            status_updater.propagate_pending_status("Starting rollup");

            rollup_deployer.start().await.map_err(|e| {
                let diag = diagnosed_error!("Failed to start rollup: {e}");
                status_updater.propagate_failed_status("Failed to start rollup", &diag);
                diag
            })?;

            status_updater.propagate_success_status("Complete", "All rollup services online");

            let max_attempts = 30;
            let mut attempts = 0;
            loop {
                attempts += 1;
                if attempts == max_attempts {
                    break;
                }
                status_updater.propagate_pending_status(
                    "Waiting for rollup to be ready to receive transactions",
                );
                sleep(Duration::from_secs(1));
                // if rollup_deployer.check_ready_state().await {
                //     break;
                // }
            }
            status_updater
                .propagate_success_status("Ready", "Rollup is ready to receive transactions");

            let mut result = CommandExecutionResult::new();
            let rollup_container_ids = rollup_deployer.get_container_ids();
            result.outputs.insert(ROLLUP_CONTAINER_IDS.to_string(), rollup_container_ids.clone());
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
