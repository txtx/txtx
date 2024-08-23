use alloy::primitives::Address;
use forge_verify::provider::VerificationContext;
use forge_verify::sourcify::SourcifyVerificationProvider;
use forge_verify::{EtherscanVerificationProvider, VerifyArgs};
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusColor, StatusUpdater,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};
use txtx_addon_kit::uuid::Uuid;

use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

lazy_static! {
    pub static ref VERIFY_CONTRACT: PreCommandSpecification = define_command! {
        VerifyEVMContract => {
            name: "Broadcast Stacks Transaction",
            matcher: "verify_contract",
            documentation: "The `evm::verify_contract` action sends the required contract deployment artifacts to a block explorer to verify the contract with the explorer.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                block_explorer_opts: {
                  documentation: "The URL of the block explorer used to verify the contract.",
                  typing: define_object_type!{
                    key: {
                        documentation: "The block explorer API key.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true
                    },
                    url: {
                        documentation: "The block explorer contract verification URL (default Etherscan).",
                        typing: Type::string(),
                        optional: true,
                        tainting: true
                    }
                  },
                  optional: false,
                  tainting: true,
                  internal: false
                },
                contract_address: {
                    documentation: "The contract address to verify.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                artifacts: {
                    documentation: indoc!{ r#"An object containing the deployment artifacts. Schema:
                    ```json
                        {
                            "abi": String,
                            "bytecode": String,
                            "source": String,
                            "compiler_version": String,
                            "contract_name": String,
                            "optimizer_enabled": Bool,
                            "optimizer_runs": UInt,
                            "evn_version": String
                        }
                    ```
                    "# },
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone(),
                    optional: false,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
              result: {
                  documentation: "The contract verification result.",
                  typing: Type::buffer()
              }
            ],
            example: txtx_addon_kit::indoc! {r#"
            action "verify_contract" "evm::verify_contract" {
                contract_address = evm::address(env.MY_CONTRACT_ADDRESS)
                artifacts = action.artifacts
            }
        "#},
        }
    };
}
pub struct VerifyEVMContract;
impl CommandImplementation for VerifyEVMContract {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        //    Todo: check network consistency?
        // let network = match transaction.version {
        //     TransactionVersion::Mainnet => "mainnet".to_string(),
        //     TransactionVersion::Testnet => "testnet".to_string(),
        // };

        // let network_id = args.get("network_id")
        //     .and_then(|a| Some(a.expect_string()))
        //     .or(defaults.keys.get("network_id").map(|x| x.as_str()))
        //     .ok_or(Diagnostic::error_from_string(format!("Key 'network_id' is missing")))?;
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
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    #[cfg(not(feature = "wasm"))]
    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        _outputs: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        use alloy::{dyn_abi::DynSolValue, hex};
        use alloy_chains::Chain;
        use forge_verify::provider::{VerificationProvider, VerificationProviderType};
        use txtx_addon_kit::types::frontend::StatusUpdater;

        use crate::{
            codec::{value_to_sol_value, verify::DeploymentArtifacts},
            commands::actions::get_expected_address,
            constants::{ARTIFACTS, CHAIN_ID, CONTRACT_ADDRESS, CONTRACT_CONSTRUCTOR_ARGS},
        };

        let inputs = inputs.clone();
        let construct_did = construct_did.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let chain_id = inputs.get_expected_uint(CHAIN_ID)?;

        let contract_address = inputs.get_expected_value(CONTRACT_ADDRESS)?;
        let explorer_opts = inputs.get_expected_object("block_explorer_opts")?;
        let (explorer_key, explorer_url) = match (explorer_opts.get("key"), explorer_opts.get("url")) {
            (None, None) => return Err(diagnosed_error!("command 'evm::verify_contract': block explorer options must include block explorer API key or URL.")),
            (Some(key), Some(url)) => (key.as_string().and_then(|k| Some(k.to_string())),  url.as_string().and_then(|u| Some(u.to_string()))),
            (Some(key), None) => (key.as_string().and_then(|k| Some(k.to_string())),  None),
            (None, Some(url)) => (None,  url.as_string().and_then(|u| Some(u.to_string()))),
        };

        let artifacts: DeploymentArtifacts =
            DeploymentArtifacts::from_value(inputs.get_expected_value(ARTIFACTS)?)
                .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {}", e))?;

        let constructor_args =
            if let Some(function_args) = inputs.get_value(CONTRACT_CONSTRUCTOR_ARGS) {
                let sol_args = function_args
                    .expect_array()
                    .iter()
                    .map(|v| {
                        value_to_sol_value(&v)
                            .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {}", e))
                    })
                    .collect::<Result<Vec<DynSolValue>, Diagnostic>>()?
                    .iter()
                    .flat_map(|s| s.abi_encode())
                    .collect::<Vec<u8>>();
                Some(hex::encode(&sol_args))
            } else {
                None
            };

        let contract_address = get_expected_address(&contract_address).map_err(|e| {
            diagnosed_error!("command 'evm::verify_contract' failed to parse contract address: {e}")
        })?;

        let mut status_updater =
            StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

        let future = async move {
            let result = CommandExecutionResult::new();

            status_updater.propagate_pending_status(&submitting_msg(&contract_address.to_string()));

            let chain = Chain::from(chain_id);

            let verify_args = artifacts
                .to_verify_args(
                    contract_address,
                    constructor_args,
                    chain,
                    explorer_key,
                    explorer_url,
                )
                .unwrap();

            let verification_context = artifacts.to_verification_context().unwrap();

            let verifier_type = &verify_args.verifier.verifier;
            let is_etherscan_provider = verifier_type == &VerificationProviderType::Etherscan
                || verifier_type == &VerificationProviderType::Blockscout
                || verifier_type == &VerificationProviderType::Oklink;

            let verify_result = if is_etherscan_provider {
                verify_etherscan_type_provider(
                    verify_args,
                    verification_context,
                    &mut status_updater,
                    contract_address,
                )
                .await
                .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {}", e))
            } else {
                verify_sourcify_type_provider(
                    verify_args,
                    verification_context,
                    &mut status_updater,
                    contract_address,
                )
                .await
                .map_err(|e| diagnosed_error!("command 'evm::verify_contract': {}", e))
            };

            if let Err(diag) = verify_result {
                status_updater
                    .propagate_status(verify_failed_status(&contract_address.to_string(), &diag));
                return Err(diag);
            }

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}

async fn verify_etherscan_type_provider(
    verify_args: VerifyArgs,
    verification_context: VerificationContext,
    status_updater: &mut StatusUpdater,
    contract_address: Address,
) -> Result<(), String> {
    let contract_address_str = contract_address.to_string();
    let err_prefix = format!("failed to verify contract {}", contract_address_str);

    let mut verification_provider = EtherscanVerificationProvider::default();
    let (etherscan, verify_contract) = verification_provider
        .prepare_request(&verify_args, &verification_context)
        .await
        .map_err(|e| format!("{}: failed to prepare verification request: {}", err_prefix, e))?;

    let submitting_msg = submitting_msg(&contract_address_str);
    let checking_msg = checking_msg(&contract_address_str);
    let address_url = etherscan.address_url(contract_address);
    let already_verified = already_verified_status(&address_url);
    let successfully_verified = verify_success_status(&address_url);

    let mut attempts = 0;
    let max_attempts = 40;
    let guid = loop {
        status_updater.propagate_pending_status(&submitting_msg);
        let resp = etherscan.submit_contract_verification(&verify_contract).await.unwrap();

        if resp.status == "0" {
            if resp.result == "Contract source code already verified"
                // specific for blockscout response
                || resp.result == "Smart-contract already verified."
            {
                status_updater.propagate_status(already_verified);
                return Ok(());
            }

            if attempts == max_attempts {
                return Err(format!("{}: {}", err_prefix, resp.result));
            } else {
                attempts += 1;
                sleep_ms(500);
                continue;
            }
        }
        break resp.result;
    };

    if verify_args.watch {
        let mut attempts = 0;
        let max_attempts = 10;
        loop {
            status_updater.propagate_pending_status(&checking_msg);
            let resp = etherscan.check_contract_verification_status(&guid).await.unwrap();

            if resp.result == "Pass - Verified" {
                status_updater.propagate_status(successfully_verified);
                return Ok(());
            } else if resp.result == "Already Verified" {
                status_updater.propagate_status(already_verified);
                return Ok(());
            } else if resp.result == "Pending in queue" {
                sleep_ms(500);
                continue;
            } else if resp.status == "0" {
                return Err(format!("{}: {}", err_prefix, resp.result));
            } else {
                if attempts == max_attempts {
                    return Err(format!("{}: {}", err_prefix, resp.result));
                } else {
                    attempts += 1;
                    sleep_ms(500);
                    continue;
                }
            }
        }
    }

    Ok(())
}

async fn verify_sourcify_type_provider(
    verify_args: VerifyArgs,
    verification_context: VerificationContext,
    status_updater: &mut StatusUpdater,
    contract_address: Address,
) -> Result<(), String> {
    let contract_address_str = contract_address.to_string();
    let err_prefix = format!("failed to verify contract {}", contract_address_str);

    let verification_provider = SourcifyVerificationProvider::default();
    let sourcify_req =
        verification_provider.prepare_request(&verify_args, &verification_context).map_err(
            |e| format!("{}: failed to prepare verification request: {}", err_prefix, e),
        )?;

    let client = txtx_addon_kit::reqwest::Client::new();

    let sourcify_url = verify_args.verifier.verifier_url.as_deref().unwrap();

    let submitting_msg = submitting_msg(&contract_address_str);
    let address_url = format!("{}/address/{}", sourcify_url, contract_address_str);
    let successfully_verified = verify_success_status(&address_url);

    let mut attempts = 0;
    let max_attempts = 40;
    let verification_response = loop {
        status_updater.propagate_pending_status(&submitting_msg);
        let response = client
            .post(sourcify_url)
            .header("Content-Type", "application/json")
            .body(
                serde_json::to_string(&sourcify_req).map_err(|e| {
                    format!("failed to serialize contract verification request: {}", e)
                })?,
            )
            .send()
            .await
            .map_err(|e| format!("failed to send sourcify contract verification request: {e}"))?;

        status_updater.propagate_pending_status(&submitting_msg);
        let status = response.status();
        println!("\nresponse: {:?}\nstatus:{:?}", response, status);
        if !status.is_success() {
            let error: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("failed to parse sourcify error response: {e}"))?;
            if attempts == max_attempts {
                return Err(format!(
                    "{}: sourcify verification for address {} failed: {error:#}",
                    err_prefix, contract_address_str
                ));
            } else {
                attempts += 1;
                sleep_ms(500);
                continue;
            }
        }

        let text =
            response.text().await.map_err(|e| format!("failed to parse sourcify response: {e}"))?;
        status_updater.propagate_pending_status(&submitting_msg);
        println!("\nsourcify resp: {}", text);
        let res = serde_json::from_str::<SourcifyVerificationResponse>(&text)
            .map_err(|e| format!("unexpected sourcify response: {e}"))?;

        break res;
    };

    let SourcifyResponseElement { response } = verification_response
        .result
        .first()
        .ok_or(format!("received no response from sourcify verification request"))?;
    match response {
        SourcifyResponse::Ok(response) => match response.status.as_str() {
            "perfect" => {
                status_updater.propagate_status(successfully_verified);

                return Ok(());
            }
            "partial" => {
                return Err(format!(
                "the recompiled contract partially matches the deployed version for contract {}",
                contract_address_str
            ));
            }
            s => {
                return Err(format!(
                    "unknown status {} from sourcify for contract {}",
                    s, contract_address_str
                ))
            }
        },
        SourcifyResponse::Err(response) => {
            return Err(format!(
                "sourcify contract verification failed for contract {}: {}",
                contract_address_str, response.message
            ))
        }
    }
}

fn verify_failed_status(address: &str, diag: &Diagnostic) -> ProgressBarStatus {
    ProgressBarStatus::new_err(
        "Failed",
        &format!("Contract Verification Failed for Contract {}", address),
        diag,
    )
}
fn verify_success_status(url: &str) -> ProgressBarStatus {
    ProgressBarStatus::new_msg(
        ProgressBarStatusColor::Green,
        "Verified",
        &format!("Contract Successfully Verified at {}", url),
    )
}
fn already_verified_status(url: &str) -> ProgressBarStatus {
    ProgressBarStatus::new_msg(
        ProgressBarStatusColor::Green,
        "Verified",
        &format!("Contract Already Verified at {}", url),
    )
}
fn submitting_msg(address: &str) -> String {
    format!("Submitting Contract {} to Explorer for Verification", address)
}
fn checking_msg(address: &str) -> String {
    format!("Checking Verification Status for Contract {}", address)
}

// Copied from foundry crate. The status field should be Optional, but foundry had it as required
#[derive(Debug, Deserialize)]
pub struct SourcifyVerificationResponse {
    pub result: Vec<SourcifyResponseElement>,
}
#[derive(Debug, Deserialize)]
pub struct SourcifyOkResponse {
    status: String,
}
#[derive(Debug, Deserialize)]

pub struct SourcifyErrResponse {
    message: String,
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SourcifyResponse {
    Ok(SourcifyOkResponse),
    Err(SourcifyErrResponse),
}
#[derive(Debug, Deserialize)]
pub struct SourcifyResponseElement {
    #[serde(flatten)]
    response: SourcifyResponse,
}
