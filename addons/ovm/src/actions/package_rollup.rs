use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::{types::RunbookSupervisionContext, ConstructDid};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::docker::RollupPackager;
use crate::constants::{ROLLUP_CONTAINER_IDS, WORKING_DIR};
use crate::typing::ROLLUP_CONTAINER_IDS_TYPE;

lazy_static! {
    pub static ref PACKAGE_ROLLUP: PreCommandSpecification = define_command! {
        PackageRollup => {
            name: "Coming Soon",
            matcher: "package_rollup",
            documentation: "The `ovm::package_rollup` action is coming soon.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                working_dir: {
                    documentation: "Coming soon.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                rollup_container_ids: {
                    documentation: "Coming soon.",
                    typing: ROLLUP_CONTAINER_IDS_TYPE.clone(),
                    optional: false,
                    tainting: false,
                    internal: false
                }
            ],
            outputs: [
                genesis: {
                    documentation: "The genesis.json settings.",
                    typing: Type::arbitrary_object()
                },
                rollup: {
                    documentation: "The rollup.json settings.",
                    typing: Type::arbitrary_object()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                // Coming soon
            "#},
        }
    };
}

pub struct PackageRollup;
impl CommandImplementation for PackageRollup {
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
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        _construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        _outputs: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
        _cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        let inputs = inputs.clone();

        let future = async move {
            let working_dir = inputs.get_expected_string(WORKING_DIR)?;
            let rollup_container_ids = inputs.get_expected_object(ROLLUP_CONTAINER_IDS)?;

            let rollup_packager = RollupPackager::new(working_dir, rollup_container_ids)
                .map_err(|e| diagnosed_error!("Failed to package rollup: {e}"))?;

            // let mut status_updater =
            //     StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

            // status_updater.propagate_pending_status(
            //     "Pausing, packaging, and removing rollup from Docker network",
            // );

            rollup_packager.package_rollup().await.map_err(|e| {
                let diag = diagnosed_error!("Failed to package rollup: {e}");
                // status_updater.propagate_failed_status("Failed to initialize rollup", &diag);
                diag
            })?;

            // status_updater.propagate_success_status(
            //     "Complete",
            //     &format!("Rollup packaged successfully - files are available at {}", working_dir),
            // );
            let result = CommandExecutionResult::new();

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
