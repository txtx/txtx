use std::{collections::HashMap, fs, path::PathBuf};

use alloy::primitives::Address;
use alloy_chains::Chain;
use foundry_compilers::{multi::MultiCompilerParser, Graph};
use foundry_compilers_artifacts_solc::Source;
// use foundry_config::ethers_solc::{artifacts::Source, Graph};
use txtx_addon_kit::{
    reqwest::{Client, Url},
    types::diagnostics::Diagnostic,
};

use crate::codec::{
    contract_deployment::compiled_artifacts::CompiledContractArtifacts, verify::Provider,
};

use super::{CheckVerificationStatusResult, SubmitVerificationResult, Verifier};

pub struct SourcifyVerificationClient {
    pub client: Client,
    pub chain: Chain,
    pub provider_api_url: Url,
    pub provider_url: Option<Url>,
    pub address: Address,
}
impl SourcifyVerificationClient {
    fn submit_verification_url(&self) -> String {
        if Provider::is_default_sourcify_api_url(&self.provider_api_url) {
            format!("{}/server/verify", self.provider_api_url)
        } else {
            self.provider_api_url.to_string()
        }
    }

    fn check_verification_status_url(&self) -> String {
        if Provider::is_default_sourcify_api_url(&self.provider_api_url) {
            format!(
                "{}/server/v2/contract/{}/{}",
                self.provider_api_url.to_string(),
                self.chain.id(),
                self.address
            )
        } else {
            self.provider_api_url.to_string()
        }
    }
}

impl Verifier for SourcifyVerificationClient {
    fn new(
        _api_key: &str,
        provider_api_url: &Url,
        provider_url: &Option<Url>,
        chain: Chain,
        address_bytes: &Vec<u8>,
    ) -> Result<Self, Diagnostic>
    where
        Self: Sized,
    {
        let address = Address::from_slice(address_bytes);
        let client = Client::new();
        Ok(Self {
            client,
            chain,
            provider_api_url: provider_api_url.clone(),
            provider_url: provider_url.clone(),
            address,
        })
    }

    async fn submit_contract_verification(
        &self,
        deployment_artifacts: &CompiledContractArtifacts,
        _constructor_args: &Option<String>,
    ) -> Result<SubmitVerificationResult, Diagnostic> {
        let args = SourcifyVerifyRequest::new(deployment_artifacts, &self.address, self.chain.id())
            .map_err(|diag| {
                diagnosed_error!("failed to create sourcify verification args: {}", diag)
            })?;

        let args = serde_json::to_string(&args).map_err(|e| {
            diagnosed_error!("failed to serialize sourcify verification args: {}", e)
        })?;

        let res = self
            .client
            .post(self.submit_verification_url())
            .header("Content-Type", "application/json")
            .body(args)
            .send()
            .await
            .map_err(|e| diagnosed_error!("failed to send contract verification request: {}", e))?;

        let status = res.status();
        if !status.is_success() {
            let err = res.json::<SourcifyVerificationError>().await.map_err(|e| {
                diagnosed_error!(
                    "failed to parse response from submitting sourcify verification: {}",
                    e
                )
            })?;

            if err.message.eq(format!(
                "The contract {} on chainId {} is already being verified, please wait",
                self.address,
                self.chain.id()
            )
            .as_str())
            {
                return Ok(SubmitVerificationResult::CheckVerification(String::new()));
            }
            return Ok(SubmitVerificationResult::NotVerified(err.message));
        }

        let text = res.text().await.map_err(|e| {
            diagnosed_error!("failed to read sourcify verification response: {}", e)
        })?;

        let response =
            serde_json::from_str::<SourcifyVerificationResponse>(&text).map_err(|e| {
                diagnosed_error!("failed to parse sourcify verification response: {}", e)
            })?;

        let result = response.result.first().ok_or(diagnosed_error!(
            "failed to parse sourcify verification response: {}",
            "no result found"
        ))?;

        match result.status.as_str() {
            "perfect" => {
                return Ok(SubmitVerificationResult::Verified);
            }
            "partial" => {
                return Ok(SubmitVerificationResult::PartiallyVerified);
            }
            "false" => {
                return Ok(SubmitVerificationResult::NotVerified(
                    "Contract source code is not verified".into(),
                ))
            }
            s => return Err(diagnosed_error!("Unknown status from sourcify. Status: {s:?}")),
        }
    }

    async fn check_contract_verification_status(
        &self,
        _guid: &str,
    ) -> Result<CheckVerificationStatusResult, Diagnostic> {
        let res =
            self.client.get(self.check_verification_status_url()).send().await.map_err(|e| {
                diagnosed_error!(
                    "failed to send request to verify contract verification status: {}",
                    e
                )
            })?;
        let status = res.status();
        if !status.is_success() {
            let err = res.json::<SourcifyVerificationError>().await.map_err(|e| {
                diagnosed_error!(
                    "failed to parse response when checking sourcify verification status: {}",
                    e
                )
            })?;
            return Ok(CheckVerificationStatusResult::NotVerified(err.message));
        }

        let text = res.text().await.map_err(|e| {
            diagnosed_error!("failed to read sourcify verification status check response: {}", e)
        })?;

        let response =
            serde_json::from_str::<SourcifyCheckContractResponse>(&text).map_err(|e| {
                diagnosed_error!(
                    "failed to parse sourcify verification status check response: {}",
                    e
                )
            })?;

        match response.match_state {
            SourcifyMatchState::Match => Ok(CheckVerificationStatusResult::Verified),
            SourcifyMatchState::ExactMatch => Ok(CheckVerificationStatusResult::Verified),
        }
    }

    fn get_address_url(&self) -> Option<String> {
        if let Some(provider_url) = self.provider_url.as_ref() {
            return Some(provider_url.to_string());
        }
        None
    }
}

// Note: copied from the foundry/verify repo (not published as crate)
// https://github.com/foundry-rs/foundry/blob/3e9385b65d5ff502095c7896aab6042127548c34/crates/verify/src/sourcify.rs#L160
#[derive(Debug, Serialize)]
pub struct SourcifyVerifyRequest {
    address: String,
    chain: String,
    files: HashMap<String, String>,
    #[serde(rename = "chosenContract", skip_serializing_if = "Option::is_none")]
    chosen_contract: Option<String>,
}

impl SourcifyVerifyRequest {
    pub fn new(
        artifacts: &CompiledContractArtifacts,
        contract_address: &Address,
        chain_id: u64,
    ) -> Result<Self, Diagnostic> {
        let project = artifacts.project()?;

        let contract_target_path = artifacts.contract_target_path_buf()?;

        let source_path = contract_target_path
            .strip_prefix(project.root())
            .map_err(|e| {
                diagnosed_error!("failed to strip project root from contract target path: {}", e)
            })?
            .to_path_buf();

        // load all contract source code from foundry project
        let mut sources = project.paths.read_input_files().map_err(|e| {
            diagnosed_error!("failed to read input files from project paths: {}", e)
        })?;

        // load the contract source code from the target path
        sources.insert(
            source_path.clone(),
            Source::read(&source_path)
                .map_err(|e| diagnosed_error!("failed to read source file: {}", e))?,
        );

        // traverse the dependency graph to find all imports
        let graph = Graph::<MultiCompilerParser>::resolve_sources(&project.paths, sources)
            .map_err(|e| diagnosed_error!("failed to resolve sources: {}", e))?;

        // get the imports for the contract target path
        let imports: Vec<PathBuf> =
            graph.imports(&source_path).into_iter().map(Into::into).collect();

        let mut files = HashMap::with_capacity(2 + imports.len());

        let metadata = artifacts.metadata.as_ref().ok_or_else(|| {
            diagnosed_error!("missing expected metadata in compiled contract artifacts")
        })?;

        let metadata = serde_json::to_string(&metadata)
            .map_err(|e| diagnosed_error!("failed to serialize metadata: {}", e))?;

        files.insert("metadata.json".to_string(), metadata);

        files.insert(
            source_path
                .file_name()
                .ok_or_else(|| {
                    diagnosed_error!("failed to get file name from contract target path")
                })?
                .to_string_lossy()
                .to_string(),
            fs::read_to_string(&source_path)
                .map_err(|e| diagnosed_error!("failed to read contract file: {}", e))?,
        );

        for import in imports {
            files.insert(
                import.display().to_string(),
                fs::read_to_string(&import)
                    .map_err(|e| diagnosed_error!("failed to read import file: {}", e))?,
            );
        }

        Ok(Self {
            address: contract_address.to_string(),
            chain: chain_id.to_string(),
            files,
            chosen_contract: None,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SourcifyVerificationResponse {
    pub result: Vec<SourcifyResponseElement>,
}

#[derive(Debug, Deserialize)]
pub struct SourcifyResponseElement {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SourcifyVerificationError {
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct SourcifyCheckContractResponse {
    #[serde(rename = "match")]
    match_state: SourcifyMatchState,
}
#[derive(Debug, Deserialize)]
pub enum SourcifyMatchState {
    Match,
    ExactMatch,
}
