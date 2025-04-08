use crate::codec::{
    contract_deployment::compiled_artifacts::CompiledContractArtifacts, value_to_sol_value,
};
use crate::constants::{
    CHAIN_ID, CONTRACT, CONTRACT_ADDRESS, CONTRACT_CONSTRUCTOR_ARGS, CONTRACT_VERIFICATION_OPTS,
};
use crate::typing::EvmValue;
use alloy::dyn_abi::JsonAbiExt;
use alloy::{dyn_abi::DynSolValue, hex};
use alloy_chains::Chain;
use providers::{CheckVerificationStatusResult, SubmitVerificationResult, VerificationClient};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use txtx_addon_kit::reqwest::Url;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{
    BlockEvent, ProgressBarStatus, ProgressBarStatusColor, StatusUpdater,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{ObjectType, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use super::value_to_abi_constructor_args;

pub mod providers;

const VERIFIED: &str = "verified";
const URL: &str = "url";
const ERROR: &str = "error";
const PROVIDER: &str = "provider";

pub async fn verify_contracts(
    construct_did: &ConstructDid,
    inputs: &ValueStore,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    background_tasks_uuid: &Uuid,
) -> Result<Value, Diagnostic> {
    let mut status_updater =
        StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

    let chain_id = inputs.get_expected_uint(CHAIN_ID)?;

    let contract_address = EvmValue::to_address(inputs.get_expected_value(CONTRACT_ADDRESS)?)?;
    let contract_address_str = contract_address.to_string();

    let Some(contract_verification_opts) = inputs.get_map(CONTRACT_VERIFICATION_OPTS) else {
        status_updater.propagate_status(verify_skipped_status(&contract_address_str));
        return Ok(Value::array(vec![]));
    };

    let contract_verification_opts =
        ContractVerificationOpts::from_values(contract_verification_opts.to_vec(), chain_id)?;

    let artifacts: CompiledContractArtifacts =
        CompiledContractArtifacts::from_map(&inputs.get_expected_object(CONTRACT)?)?;

    let constructor_args = if let Some(function_args) = inputs.get_value(CONTRACT_CONSTRUCTOR_ARGS)
    {
        let sol_args = if let Some(abi) = &artifacts.abi {
            if let Some(constructor) = &abi.constructor {
                let sol_args = value_to_abi_constructor_args(&function_args, &constructor)?;
                constructor
                    .abi_encode_input(&sol_args)
                    .map_err(|e| diagnosed_error!("failed to encode constructor args: {}", e))?
            } else {
                return Err(diagnosed_error!(
                    "constructor args provided, but no constructor found in abi"
                ));
            }
        } else {
            let sol_args = function_args
                .expect_array()
                .iter()
                .map(|v| {
                    value_to_sol_value(&v)
                        .map_err(|e| diagnosed_error!("failed to encode constructor args: {}", e))
                })
                .collect::<Result<Vec<DynSolValue>, Diagnostic>>()?;

            sol_args.iter().flat_map(|s| s.abi_encode()).collect::<Vec<u8>>()
        };
        Some(hex::encode(&sol_args))
    } else {
        None
    };

    let mut contract_verification_results = vec![];

    let chain = Chain::from(chain_id);

    // track failures for each provider, so we can run each to completion and log or return errors afterwards
    let mut failures = vec![];
    for (i, opts) in contract_verification_opts.iter().enumerate() {
        let ContractVerificationOpts { provider, .. } = opts;
        let err_ctx = format!(
            "contract verification failed for contract '{}' with provider '{}'",
            contract_address_str,
            provider.to_string()
        );
        let mut result_for_explorer =
            ObjectType::from([(PROVIDER, Value::string(provider.to_string()))]);

        failures.insert(i, None);

        let client = match VerificationClient::new(opts, chain, &contract_address) {
            Ok(client) => client,
            Err(diag) => {
                propagate_failed_status(
                    &mut status_updater,
                    &contract_address_str,
                    &provider,
                    &diag,
                );
                result_for_explorer.insert(ERROR, Value::string(diag.to_string()));
                result_for_explorer.insert(VERIFIED, Value::bool(false));
                contract_verification_results.push(result_for_explorer.to_value());
                failures[i] = Some(diagnosed_error!("{}: {}", err_ctx, diag));
                continue;
            }
        };

        let max_attempts = 10;
        let mut attempts = 0;
        let guid = loop {
            attempts += 1;

            propagate_submitting_status(&mut status_updater, &contract_address_str, &provider);

            let verification_result =
                match client.submit_contract_verification(&artifacts, &constructor_args).await {
                    Ok(res) => res,
                    Err(diag) => {
                        propagate_failed_status(
                            &mut status_updater,
                            &contract_address_str,
                            &provider,
                            &diag,
                        );
                        result_for_explorer.insert(ERROR, Value::string(diag.to_string()));
                        result_for_explorer.insert(VERIFIED, Value::bool(false));
                        contract_verification_results.push(result_for_explorer.to_value());
                        failures[i] = Some(diagnosed_error!("{}: {}", err_ctx, diag));
                        continue;
                    }
                };
            verification_result.propagate_status(
                &mut status_updater,
                &client,
                max_attempts == attempts, // propagate errors if this is our last attempt
            );

            match verification_result {
                SubmitVerificationResult::CheckVerification(guid) => break Some(guid),
                SubmitVerificationResult::NotVerified(err) => {
                    if attempts == max_attempts {
                        let diag = diagnosed_error!("{}: {}", err_ctx, err);
                        result_for_explorer.insert(ERROR, Value::string(diag.to_string()));
                        result_for_explorer.insert(VERIFIED, Value::bool(false));
                        contract_verification_results.push(result_for_explorer.to_value());
                        failures[i] = Some(diag);
                        break None;
                    } else {
                        sleep_ms(2000);
                        continue;
                    }
                }
                _ => {
                    result_for_explorer.insert(URL, Value::string(client.address_url()));
                    result_for_explorer.insert(VERIFIED, Value::bool(true));
                    contract_verification_results.push(result_for_explorer.to_value());
                    break None;
                }
            };
        };

        let guid = match guid {
            Some(guid) => guid,
            None => continue,
        };

        let max_attempts = 10;
        let mut attempts = 0;
        loop {
            attempts += 1;

            checking_status(&mut status_updater, &contract_address_str, &provider);

            let res = match client.check_contract_verification_status(&guid).await {
                Ok(res) => res,
                Err(diag) => {
                    propagate_failed_status(
                        &mut status_updater,
                        &contract_address_str,
                        &provider,
                        &diag,
                    );
                    result_for_explorer.insert(ERROR, Value::string(diag.to_string()));
                    result_for_explorer.insert(VERIFIED, Value::bool(false));
                    contract_verification_results.push(result_for_explorer.to_value());
                    failures[i] = Some(diagnosed_error!("{}: {}", err_ctx, diag));
                    break;
                }
            };

            res.propagate_status(
                &mut status_updater,
                &client,
                max_attempts == attempts, // propagate errors if this is our last attempt
            );

            match res {
                CheckVerificationStatusResult::NotVerified(err) => {
                    if max_attempts == attempts {
                        let diag = diagnosed_error!("{}: {}", err_ctx, err);
                        result_for_explorer.insert(ERROR, Value::string(diag.to_string()));
                        result_for_explorer.insert(VERIFIED, Value::bool(false));
                        contract_verification_results.push(result_for_explorer.to_value());
                        failures[i] = Some(diag);
                        break;
                    } else {
                        sleep_ms(2000);
                        continue;
                    }
                }
                _ => {
                    result_for_explorer.insert(VERIFIED, Value::bool(true));
                    result_for_explorer.insert(URL, Value::string(client.address_url()));
                    contract_verification_results.push(result_for_explorer.to_value());
                    break;
                }
            }
        }
    }

    for (opt, failure) in contract_verification_opts.iter().zip(failures) {
        if let Some(diag) = failure {
            if opt.throw_on_error {
                return Err(diag);
            }
        }
    }
    Ok(Value::array(contract_verification_results))
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}

fn verify_skipped_status(address: &str) -> ProgressBarStatus {
    ProgressBarStatus::new_msg(
        ProgressBarStatusColor::Yellow,
        "Verification Skipped",
        &format!("Skipping verification for contract {}; no verifier opts provided", address),
    )
}

fn propagate_failed_status(
    status_updater: &mut StatusUpdater,
    address: &str,
    provider: &Provider,
    diag: &Diagnostic,
) {
    status_updater.propagate_status(ProgressBarStatus::new_err(
        "Verification Failed",
        &format!(
            "Verification failed for contract '{}' and provider '{}'",
            address,
            provider.to_string()
        ),
        diag,
    ));
}

fn propagate_submitting_status(
    status_updater: &mut StatusUpdater,
    address: &str,
    provider: &Provider,
) {
    status_updater.propagate_pending_status(&format!(
        "Submitting contract '{}' for verification by provider '{}'",
        address,
        provider.to_string()
    ));
}

fn checking_status(status_updater: &mut StatusUpdater, address: &str, provider: &Provider) {
    status_updater.propagate_pending_status(&format!(
        "Checking verification status for contract '{}' with provider '{}'",
        address,
        provider.to_string()
    ));
}

pub struct ContractVerificationOpts {
    pub provider_api_url: Url,
    pub provider_url: Url,
    pub api_key: Option<String>,
    pub provider: Provider,
    pub throw_on_error: bool,
}

impl ContractVerificationOpts {
    pub fn from_values(opts_values: Vec<Value>, chain_id: u64) -> Result<Vec<Self>, Diagnostic> {
        let mut opts = vec![];

        for opts_value in opts_values {
            let object = opts_value
                .as_object()
                .ok_or(diagnosed_error!("each verifier map must be an object"))?;

            let provider = object
                .get("provider")
                .ok_or(diagnosed_error!("verifier object must contain the 'provider' key"))?
                .as_string()
                .ok_or(diagnosed_error!("'provider' in verifier options must be a string"))?;

            let provider = Provider::from_str(provider)?;

            let provider_api_url = object
                .get("provider_api_url")
                .map(|explorer_api_url| {
                    explorer_api_url.as_string().ok_or(diagnosed_error!(
                        "'provider_api_url' in verifier options must be a string"
                    ))
                })
                .transpose()?;

            let provider_api_url = if let Some(provider_api_url) = provider_api_url {
                Url::parse(provider_api_url)
                    .map_err(|e| diagnosed_error!("failed to parse provider_api_url: {e}"))?
            } else {
                provider.api_url(chain_id)?
            };

            let provider_url = object
                .get("provider_url")
                .map(|explorer_url| {
                    explorer_url.as_string().ok_or(diagnosed_error!(
                        "'provider_url' in verifier options must be a string"
                    ))
                })
                .transpose()?;

            let provider_url = if let Some(provider_url) = provider_url {
                Url::parse(provider_url)
                    .map_err(|e| diagnosed_error!("failed to parse provider_url: {e}"))?
            } else {
                provider.url(chain_id)?
            };

            let api_key = object
                .get("api_key")
                .map(|key| {
                    key.as_string()
                        .map(|key| key.to_string())
                        .ok_or(diagnosed_error!("'api_key' in verifier options must be a string"))
                })
                .transpose()?;

            let throw_on_error = object
                .get("throw_on_error")
                .map(|v| {
                    v.as_bool().ok_or(diagnosed_error!(
                        "'throw_on_error' in verifier options must be a boolean"
                    ))
                })
                .transpose()?
                .unwrap_or(false);

            opts.push(ContractVerificationOpts {
                provider_api_url,
                api_key,
                provider_url,
                provider,
                throw_on_error,
            });
        }

        Ok(opts)
    }
}

#[derive(Clone, Debug)]
pub enum Provider {
    Etherscan,
    Blockscout,
    Sourcify,
}

impl Provider {
    pub fn from_str(provider: &str) -> Result<Self, Diagnostic> {
        match provider {
            "etherscan" => Ok(Provider::Etherscan),
            "blockscout" => Ok(Provider::Blockscout),
            "sourcify" => Ok(Provider::Sourcify),
            _ => Err(diagnosed_error!(
                "'provider' in verifier options must be one of 'etherscan', 'blockscout', or 'sourcify'"
            )),
        }
    }

    fn url(&self, chain_id: u64) -> Result<Url, Diagnostic> {
        let chain = Chain::from_id(chain_id);

        match self {
            Provider::Etherscan => {
                Url::from_str(&Self::etherscan_url(chain).ok_or(diagnosed_error!(
                    "chain_id {chain_id} is not supported by the '{}' provider",
                    self.to_string()
                ))?)
                .map_err(|e| diagnosed_error!("failed to parse etherscan url: {e}"))
            }
            Provider::Blockscout => {
                let chain_name = chain.named().map(|c| c.as_str()).ok_or(diagnosed_error!(
                    "chain_id {chain_id} is not supported by the '{}' provider",
                    self.to_string()
                ))?;
                Url::from_str(&Self::blockscout_url(chain_name))
                    .map_err(|e| diagnosed_error!("failed to parse blockscout url: {e}"))
            }
            Provider::Sourcify => Url::from_str(&Self::sourcify_url())
                .map_err(|e| diagnosed_error!("failed to parse sourcify url: {e}")),
        }
    }

    fn api_url(&self, chain_id: u64) -> Result<Url, Diagnostic> {
        let chain = Chain::from_id(chain_id);

        match self {
            Provider::Etherscan => {
                Url::from_str(&Self::etherscan_api_url(chain).ok_or(diagnosed_error!(
                    "chain_id {chain_id} is not supported by the '{}' provider",
                    self.to_string()
                ))?)
                .map_err(|e| diagnosed_error!("failed to parse etherscan api url: {e}"))
            }
            Provider::Blockscout => {
                let chain_name = chain.named().map(|c| c.as_str()).ok_or(diagnosed_error!(
                    "chain_id {chain_id} is not supported by the '{}' provider",
                    self.to_string()
                ))?;
                Url::from_str(&Self::blockscout_api_url(chain_name))
                    .map_err(|e| diagnosed_error!("failed to parse blockscout api url: {e}"))
            }
            Provider::Sourcify => Url::from_str(&Self::sourcify_api_url())
                .map_err(|e| diagnosed_error!("failed to parse sourcify api url: {e}")),
        }
    }

    fn blockscout_url(chain_name: &str) -> String {
        match chain_name {
            "mainnet" => "https://eth.blockscout.com".into(),
            "sepolia" => "https://eth-sepolia.blockscout.com".into(),
            other => format!("https://{}.blockscout.com", other),
        }
    }

    fn blockscout_api_url(chain_name: &str) -> String {
        format!("{}/api/v2", Self::blockscout_url(chain_name))
    }

    fn etherscan_url(chain: Chain) -> Option<String> {
        chain.etherscan_urls().map(|(_, url)| url.to_string())
    }

    fn etherscan_api_url(chain: Chain) -> Option<String> {
        chain.etherscan_urls().map(|(api, _)| api.to_string())
    }

    fn sourcify_url() -> String {
        "https://repo.sourcify.dev".into()
    }

    fn sourcify_api_url() -> String {
        "https://sourcify.dev".into()
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Etherscan => write!(f, "etherscan"),
            Provider::Blockscout => write!(f, "blockscout"),
            Provider::Sourcify => write!(f, "sourcify"),
        }
    }
}
