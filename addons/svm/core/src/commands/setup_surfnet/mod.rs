mod set_account;
mod set_token_account;
mod tokens;

use serde::{Deserialize, Serialize};
use serde_json::json;
use set_account::SurfpoolAccountUpdate;
use set_token_account::SurfpoolTokenAccountUpdate;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionFutureResult, CommandExecutionResult,
    CommandImplementation, CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, StatusUpdater};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_network_svm_types::{SET_ACCOUNT_MAP, SET_TOKEN_ACCOUNT_MAP};

use crate::constants::{NETWORK_ID, RPC_API_URL};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct RpcVersionInfo {
    /// The current version of surfpool, if RPC is a surfnet
    pub surfpool_version: Option<String>,
    /// The current version of solana-core
    pub solana_core: String,
    /// first 4 bytes of the FeatureSet identifier
    pub feature_set: Option<u32>,
}

lazy_static! {
    pub static ref SETUP_SURFNET: PreCommandSpecification = {
        let command = define_command! {
            SetupSurfpool => {
                name: "Setup Surfpool",
                matcher: "setup_surfnet",
                documentation: "The `svm::setup_surfnet` action is coming soon.",
                implements_signing_capability: false,
                implements_background_task_capability: true,
                inputs: [
                    description: {
                        documentation: "A description of the setup.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    rpc_api_url: {
                        documentation: "The URL to use when making API requests.",
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    network_id: {
                        documentation: "The ID of the network type. Can be `localnet`, `devnet`, or `mainnet-beta`.",
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    set_account: {
                        documentation: "The account data to set.",
                        typing: SET_ACCOUNT_MAP.clone(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    set_token_account: {
                        documentation: "The token account data to set.",
                        typing: SET_TOKEN_ACCOUNT_MAP.clone(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    }
                ],
                outputs: [],
                example: txtx_addon_kit::indoc! {r#"
                    action "setup" "svm::setup_surfnet" {
                        set_account {
                            public_key = signer.caller.public_key
                            lamports = 999999999
                        }
                        set_token_account {
                            public_key = signer.caller.public_key
                            token = "usdc"
                            amount = 1000000
                        }
                    }
                "#},
            }
        };

        command
    };
}

pub struct SetupSurfpool;
impl CommandImplementation for SetupSurfpool {
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

    fn run_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::types::commands::CommandExecutionResult;

        let construct_did = construct_did.clone();
        let values = values.clone();
        let progress_tx = progress_tx.clone();

        let future = async move {
            let result = CommandExecutionResult::new();

            let rpc_api_url = values.get_expected_string(RPC_API_URL)?;

            let network_id = values.get_expected_string(NETWORK_ID)?;

            if !network_id.eq("localnet") {
                return Err(diagnosed_error!("`network_id` must be `localnet`"));
            }
            let rpc_client = RpcClient::new(rpc_api_url.to_string());

            let version = rpc_client
                .send::<RpcVersionInfo>(RpcRequest::Custom { method: "getVersion" }, json!([]))
                .await
                .map_err(|e| diagnosed_error!("failed to fetch RPC endpoint version: {e}"))?;
            if version.surfpool_version.is_none() {
                return Err(diagnosed_error!(
                    "RPC endpoint is not a surfnet, setup_surfnet is not supported"
                ));
            }

            let mut status_updater = StatusUpdater::new(
                &Uuid::from_bytes(
                    construct_did.0 .0[0..16].try_into().expect("Failed to convert slice to array"),
                ),
                &construct_did,
                &progress_tx,
            );

            let account_updates = SurfpoolAccountUpdate::parse_value_store(&values)?;
            SurfpoolAccountUpdate::process_updates(
                account_updates,
                &rpc_client,
                &mut status_updater,
            )
            .await?;

            let token_account_updates = SurfpoolTokenAccountUpdate::parse_value_store(&values)?;
            SurfpoolTokenAccountUpdate::process_updates(
                token_account_updates,
                &rpc_client,
                &mut status_updater,
            )
            .await?;

            Ok(result)
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        _construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _outputs: &ValueStore,
        _progress_tx: &channel::Sender<BlockEvent>,
        _background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        return_synchronous_ok(CommandExecutionResult::new())
    }
}
