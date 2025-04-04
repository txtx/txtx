pub mod etherscan;
pub mod sourcify;

use alloy::primitives::Address;
use alloy_chains::Chain;
use etherscan::EtherscanVerificationClient;
use sourcify::SourcifyVerificationClient;
use txtx_addon_kit::{
    reqwest::Url,
    types::{diagnostics::Diagnostic, frontend::StatusUpdater},
};

use crate::codec::contract_deployment::compiled_artifacts::CompiledContractArtifacts;

use super::{ContractVerificationOpts, Provider};

pub enum SubmitVerificationResult {
    Verified,
    CheckVerification(String),
    NotVerified(String),
    AlreadyVerified,
    PartiallyVerified,
}

impl SubmitVerificationResult {
    /// For each verification result, propagate an associated message to the status updater.
    pub fn propagate_status(
        &self,
        status_updater: &mut StatusUpdater,
        client: &VerificationClient,
    ) {
        match &self {
            SubmitVerificationResult::Verified => {
                status_updater.propagate_success_status(
                    "Verified",
                    &format!("Contract successfully verified at {}", client.address_url()),
                );
            }
            SubmitVerificationResult::NotVerified(_) => {}
            SubmitVerificationResult::AlreadyVerified => {
                status_updater.propagate_success_status(
                    "Verified",
                    &format!("Contract already verified at {}", client.address_url()),
                );
            }
            SubmitVerificationResult::PartiallyVerified => {
                status_updater.propagate_success_status(
                    "Partially Verified",
                    &format!("Contract partially verified at {}", client.address_url()),
                );
            }
            SubmitVerificationResult::CheckVerification(_) => {
                status_updater.propagate_pending_status(&format!(
                    "Checking verification status for contract '{}' with provider '{}'",
                    client.address(),
                    client.provider()
                ));
            }
        }
    }
}

pub enum CheckVerificationStatusResult {
    Verified,
    NotVerified(String),
    AlreadyVerified,
}

impl CheckVerificationStatusResult {
    pub fn propagate_status(
        &self,
        status_updater: &mut StatusUpdater,
        client: &VerificationClient,
    ) {
        match &self {
            CheckVerificationStatusResult::Verified => {
                status_updater.propagate_success_status(
                    "Verified",
                    &format!("Contract successfully verified at {}", client.address_url()),
                );
            }
            CheckVerificationStatusResult::NotVerified(_) => {}
            CheckVerificationStatusResult::AlreadyVerified => {
                status_updater.propagate_success_status(
                    "Verified",
                    &format!("Contract already verified at {}", client.address_url()),
                );
            }
        }
    }
}

pub enum VerificationClient {
    Etherscan(EtherscanVerificationClient),
    Blockscout,
    Sourcify(SourcifyVerificationClient),
}

impl VerificationClient {
    pub fn new(
        opts: &ContractVerificationOpts,
        chain: Chain,
        contract_address: &Address,
    ) -> Result<Self, Diagnostic> {
        match opts.provider {
            Provider::Etherscan => {
                let client = EtherscanVerificationClient::new(
                    &opts
                        .api_key
                        .clone()
                        .ok_or(diagnosed_error!("'etherscan' provider requires an api_key"))?,
                    &opts.provider_api_url,
                    &opts.provider_url,
                    chain,
                    contract_address,
                )
                .map_err(|e| diagnosed_error!("failed to create etherscan client: {e}"))?;
                Ok(VerificationClient::Etherscan(client))
            }
            Provider::Blockscout => Ok(VerificationClient::Blockscout),
            Provider::Sourcify => Ok(VerificationClient::Sourcify(
                SourcifyVerificationClient::new(
                    "", // no api_key needed for sourcify
                    &opts.provider_api_url,
                    &opts.provider_url,
                    chain,
                    contract_address,
                )
                .map_err(|e| diagnosed_error!("failed to create sourcify client: {e}"))?,
            )),
        }
    }

    pub async fn submit_contract_verification(
        &self,
        deployment_artifacts: &CompiledContractArtifacts,
        constructor_args: &Option<String>,
    ) -> Result<SubmitVerificationResult, Diagnostic> {
        match self {
            VerificationClient::Etherscan(client) => {
                client.submit_contract_verification(deployment_artifacts, constructor_args).await
            }
            VerificationClient::Blockscout => unimplemented!(
                "Blockscout verification is not implemented yet. Please use etherscan or sourcify."
            ),
            VerificationClient::Sourcify(client) => {
                client.submit_contract_verification(deployment_artifacts, constructor_args).await
            }
        }
    }

    pub async fn check_contract_verification_status(
        &self,
        guid: &str,
    ) -> Result<CheckVerificationStatusResult, Diagnostic> {
        match self {
            VerificationClient::Etherscan(client) => {
                client.check_contract_verification_status(guid,).await
            }
            VerificationClient::Blockscout => unimplemented!(
                "Blockscout verification status check is not implemented yet. Please use etherscan or sourcify."
            ),
            VerificationClient::Sourcify(client) => {
                client.check_contract_verification_status(guid,).await
            }
        }
    }

    pub fn address_url(&self) -> String {
        match self {
            VerificationClient::Etherscan(client) => client.get_address_url(),
            VerificationClient::Blockscout => "".to_string(),
            VerificationClient::Sourcify(client) => client.get_address_url(),
        }
    }

    pub fn address(&self) -> Address {
        match self {
            VerificationClient::Etherscan(client) => client.address.clone(),
            VerificationClient::Blockscout => Address::new([0; 20]),
            VerificationClient::Sourcify(client) => client.address.clone(),
        }
    }

    pub fn provider(&self) -> Provider {
        match self {
            VerificationClient::Etherscan(_) => Provider::Etherscan,
            VerificationClient::Blockscout => Provider::Blockscout,
            VerificationClient::Sourcify(_) => Provider::Sourcify,
        }
    }
}

trait Verifier {
    fn new(
        api_key: &str,
        provider_api_url: &Url,
        provider_url: &Url,
        chain: Chain,
        contract_address: &Address,
    ) -> Result<Self, Diagnostic>
    where
        Self: Sized;

    async fn submit_contract_verification(
        &self,
        deployment_artifacts: &CompiledContractArtifacts,
        constructor_args: &Option<String>,
    ) -> Result<SubmitVerificationResult, Diagnostic>;

    async fn check_contract_verification_status(
        &self,
        guid: &str,
    ) -> Result<CheckVerificationStatusResult, Diagnostic>;

    fn get_address_url(&self) -> String;
}
