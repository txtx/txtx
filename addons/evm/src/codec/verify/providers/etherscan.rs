use std::str::FromStr;

use alloy::primitives::Address;
use alloy_chains::Chain;
use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
use foundry_block_explorers::Client as EtherscanClient;
use semver::Version;
use txtx_addon_kit::{reqwest::Url, types::diagnostics::Diagnostic};

use crate::codec::contract_deployment::compiled_artifacts::CompiledContractArtifacts;
use crate::codec::verify::Provider;

use super::{CheckVerificationStatusResult, SubmitVerificationResult, Verifier};

pub struct EtherscanVerificationClient {
    pub client: EtherscanClient,
    pub address: Address,
    pub chain: Chain,
}

impl Verifier for EtherscanVerificationClient {
    fn new(
        api_key: &str,
        provider_api_url: &Url,
        provider_url: &Option<Url>,
        chain: Chain,
        address: &Address,
    ) -> Result<Self, Diagnostic> {
        let mut client_builder =
            EtherscanClient::builder().with_api_key(api_key).with_chain_id(chain.clone());

        if let Some(provider_url) = provider_url {
            client_builder = client_builder
                .with_url(provider_url.clone())
                .map_err(|e| diagnosed_error!("invalid explorer url: {}", e))?;
        }

        let client = client_builder
            .with_api_url(provider_api_url.clone())
            .map_err(|e| diagnosed_error!("invalid explorer api url: {}", e))?
            .build()
            .map_err(|e| diagnosed_error!("failed to build explorer client: {}", e))?;

        Ok(Self { client, address: address.clone(), chain })
    }

    async fn submit_contract_verification(
        &self,
        deployment_artifacts: &CompiledContractArtifacts,
        constructor_args: &Option<String>,
    ) -> Result<SubmitVerificationResult, Diagnostic> {
        let verify_contract_args =
            VerifyArgsWrapper::new(deployment_artifacts, &self.address, constructor_args.clone())
                .map_err(|diag| {
                    diagnosed_error!("failed to create etherscan verification args: {}", diag)
                })?
                .0;

        let res =
            self.client.submit_contract_verification(&verify_contract_args).await.map_err(|e| {
                diagnosed_error!("failed to send contract verification request: {}", e)
            })?;

        match res.message.as_str() {
            "NOTOK" => match res.result.as_str() {
                "Contract source code already verified" => {
                    Ok(SubmitVerificationResult::AlreadyVerified)
                }
                res => {
                    return Ok(SubmitVerificationResult::NotVerified(res.to_string()));
                }
            },
            "OK" => Ok(SubmitVerificationResult::CheckVerification(res.result)),
            _ => Err(diagnosed_error!(
                "failed to submit contract verification with 'etherscan' provider: {}",
                res.result
            )),
        }
    }

    async fn check_contract_verification_status(
        &self,
        guid: &str,
    ) -> Result<CheckVerificationStatusResult, Diagnostic> {
        let res =
            self.client.check_contract_verification_status(&guid).await.map_err(|e| {
                diagnosed_error!("failed to check contract verification status: {}", e)
            })?;

        match res.message.as_str() {
            "NOTOK" => {
                if res.result == "Already Verified" {
                    return Ok(CheckVerificationStatusResult::AlreadyVerified);
                } else {
                    return Ok(CheckVerificationStatusResult::NotVerified(res.result));
                }
            }
            "OK" => {
                return Ok(CheckVerificationStatusResult::Verified);
            }
            _ => {
                return Err(diagnosed_error!(
                    "failed to check contract verification status: {}",
                    res.result
                ));
            }
        }
    }

    fn get_address_url(&self) -> Option<String> {
        if Provider::is_default_etherscan_url(&self.client.etherscan_url(), self.chain) {
            return Some(format!("{}#code", self.client.address_url(self.address)));
        }
        None
    }
}

pub struct VerifyArgsWrapper(VerifyContract);

impl VerifyArgsWrapper {
    pub fn new(
        artifacts: &CompiledContractArtifacts,
        contract_address: &Address,
        constructor_args: Option<String>,
    ) -> Result<Self, Diagnostic> {
        let contract_name = artifacts
            .contract_name
            .as_ref()
            .ok_or_else(|| diagnosed_error!("contract name required to verify contract"))?;

        let metadata = artifacts.metadata.as_ref().ok_or_else(|| {
            diagnosed_error!("compiled output metadata required to verify contract")
        })?;

        let prefixed_compiler_version = format!("v{}", metadata.compiler.version);
        let compiler_version = Version::from_str(&metadata.compiler.version)
            .map_err(|e| diagnosed_error!("invalid compiler version: {}", e))?;

        let has_remappings = !metadata.settings.remappings.is_empty();

        // if remappings are present, use standard json input, otherwise use single file
        let mut verify_contract = if has_remappings {
            // standard json input requires traversing the graph of imports. this is pretty complex,
            // so we'll rely on the foundry config/crate to do this
            let project = artifacts.project()?;

            let contract_target_path = artifacts.contract_target_path_buf()?;

            let mut input = project.standard_json_input(&contract_target_path).map_err(|e| {
                diagnosed_error!(
                    "failed to create standard json input for contract target path: {}",
                    e
                )
            })?;

            // strip the project root from the file paths in the standard json input
            input.settings.libraries.libs = input
                .settings
                .libraries
                .libs
                .into_iter()
                .map(|(f, libs)| (f.strip_prefix(project.root()).unwrap_or(&f).to_path_buf(), libs))
                .collect();

            input = input.normalize_evm_version(&compiler_version);
            input.settings = input.settings.sanitized(&compiler_version);

            let stripped_contract_target_path = contract_target_path
                .strip_prefix(project.root())
                .map_err(|e| {
                    diagnosed_error!(
                        "failed to strip project root from contract target path: {}",
                        e
                    )
                })?
                .to_path_buf();

            let source = serde_json::to_string_pretty(&input)
                .map_err(|e| diagnosed_error!("failed to serialize standard json input: {}", e))?;

            let verify_contract = VerifyContract::new(
                *contract_address,
                format!("{}:{}", stripped_contract_target_path.display(), contract_name),
                source,
                prefixed_compiler_version,
            )
            .code_format(CodeFormat::StandardJsonInput);

            verify_contract
        } else {
            let mut verify_contract = VerifyContract::new(
                *contract_address,
                contract_name.to_string(),
                artifacts
                    .source
                    .as_ref()
                    .ok_or_else(|| {
                        diagnosed_error!("contract source code required to verify contract")
                    })?
                    .to_string(),
                prefixed_compiler_version,
            )
            .code_format(CodeFormat::SingleFile);

            // optimizer settings only applied for single file input
            if let Some(optimizer_enabled) = metadata.settings.optimizer.enabled {
                verify_contract = verify_contract.optimization(optimizer_enabled);
            }
            if let Some(optimizer_runs) = metadata.settings.optimizer.runs {
                verify_contract = verify_contract.runs(optimizer_runs as u32);
            }
            verify_contract
        };

        if let Some(evm_version) = metadata.settings.evm_version.as_ref() {
            verify_contract = verify_contract.evm_version(evm_version.as_str());
        }
        if let Some(via_ir) = metadata.settings.via_ir {
            verify_contract = verify_contract.via_ir(via_ir);
        }
        verify_contract.constructor_arguments = constructor_args;
        Ok(Self(verify_contract))
    }
}
