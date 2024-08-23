use std::{path::PathBuf, str::FromStr};

use alloy::primitives::Address;
use alloy_chains::Chain;
use forge_verify::{
    provider::{VerificationContext, VerificationProviderType},
    semver::Version,
    Config, ContractInfo, EtherscanOpts, EvmVersion, RetryArgs, RpcOpts, VerifierArgs, VerifyArgs,
};
use txtx_addon_kit::types::types::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeploymentArtifacts {
    pub bytecode: String,
    pub abi: Option<String>,
    pub source: Option<String>,
    pub compiler_version: Option<String>,
    pub contract_name: Option<String>,
    /// Will usually be ContractName.sol/ContractName, but in cases where there are multiple
    /// contracts per .sol file, it could be ContractName.sol/AnotherContractName
    pub contract_target_path: Option<String>,
    pub optimizer_enabled: Option<bool>,
    pub optimizer_runs: Option<usize>,
    pub evm_version: Option<EvmVersion>,
    /// Not entirely sure how this will be used yet, for now will point to the manifest
    pub project_root: Option<String>,
    pub compilation_tool: Option<CompilationTool>,
    pub via_ir: Option<bool>,
    pub foundry_config: Option<Config>,
}

impl DeploymentArtifacts {
    pub fn new(bytecode: &str) -> Self {
        let mut default = DeploymentArtifacts::default();
        default.bytecode = bytecode.to_string();
        default
    }

    pub fn to_verify_args(
        &self,
        contract_address: Address,
        constructor_args: Option<String>,
        chain: Chain,
        explorer_key: Option<String>,
        explorer_url: Option<String>,
    ) -> Result<VerifyArgs, String> {
        let contract_info = ContractInfo {
            path: self.contract_target_path.clone(),
            name: self
                .contract_name
                .clone()
                .ok_or_else(|| "deployment artifacts missing contract name")?,
        };
        let runs = self.optimizer_runs;
        let verifier = if let Some(explorer_url) = explorer_url {
            // todo
            VerifierArgs {
                verifier: if explorer_url.contains("sourcify") {
                    VerificationProviderType::Sourcify
                } else {
                    VerificationProviderType::Etherscan
                },
                verifier_url: Some(explorer_url),
            }
        } else {
            VerifierArgs::default()
        };
        let args = VerifyArgs {
            address: contract_address,
            contract: Some(contract_info),
            constructor_args,
            constructor_args_path: None,
            guess_constructor_args: false,
            compiler_version: self.compiler_version.clone(),
            num_of_optimizations: self.optimizer_enabled.and_then(|enabled| {
                if enabled {
                    runs
                } else {
                    None
                }
            }),
            flatten: false, // todo
            force: false,
            skip_is_verified_check: true,
            watch: true,
            libraries: vec![],
            root: self
                .project_root
                .as_ref()
                .and_then(|r| {
                    Some(PathBuf::from_str(&r).map_err(|e| format!("invalid project path: {e}")))
                })
                .transpose()?,
            show_standard_json_input: false,
            via_ir: self.via_ir.unwrap_or(false),
            evm_version: self.evm_version,
            etherscan: EtherscanOpts { key: explorer_key, chain: Some(chain) },
            rpc: RpcOpts::default(),
            retry: RetryArgs { retries: 10, delay: 1 },
            verifier,
        };
        Ok(args)
    }

    pub fn to_verification_context(&self) -> Result<VerificationContext, String> {
        let artifacts = self.clone();
        let err = format!("invalid deployment artifacts for verifying contract");
        let Some(target_path) =
            artifacts.contract_target_path.and_then(|p| Some(PathBuf::from_str(&p).unwrap()))
        else {
            return Err(err);
        };

        let Some(config) = artifacts.foundry_config else {
            return Err(err);
        };
        let Some(compiler_version) = artifacts.compiler_version else {
            return Err(err);
        };
        let Some(contract_name) = artifacts.contract_name else {
            return Err(err);
        };
        let compiler_version = Version::from_str(&compiler_version)
            .map_err(|e| format!("invalid compiler version: {e}"))?;
        let ctx =
            VerificationContext::new(target_path, contract_name, compiler_version, config).unwrap();
        Ok(ctx)
    }

    pub fn from_value(value: &Value) -> Result<Self, String> {
        match value {
            Value::Object(object) => {
                let Some(bytecode) = object.get("bytecode") else {
                    return Err(format!("deployment artifacts missing required 'bytecode' key"));
                };
                let mut artifacts = DeploymentArtifacts::new(&bytecode.expect_string());
                artifacts.abi =
                    object.get("abi").and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.source = object
                    .get("source")
                    .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.compiler_version = object
                    .get("compiler_version")
                    .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.contract_name = object
                    .get("contract_name")
                    .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.contract_target_path = object
                    .get("contract_target_path")
                    .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.optimizer_enabled =
                    object.get("optimizer_enabled").and_then(|v| v.as_bool());
                artifacts.optimizer_runs = object
                    .get("optimizer_runs")
                    .and_then(|v| v.as_integer().and_then(|i| Some(i as usize)));
                artifacts.evm_version = match object.get("evm_version") {
                    Some(Value::String(evm_version)) => {
                        let version = EvmVersion::from_str(&evm_version)
                            .map_err(|e| format!("invalid evm version {}: {}", evm_version, e))?;
                        Some(version)
                    }
                    _ => None,
                };
                artifacts.project_root = object
                    .get("project_root")
                    .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
                artifacts.via_ir = object.get("via_ir").and_then(|v| v.as_bool());
                artifacts.foundry_config = match object.get("foundry_config") {
                    Some(Value::Buffer(config)) => {
                        let config = serde_json::from_slice(&config)
                            .map_err(|e| format!("invalid foundry config: {}", e))?;
                        Some(config)
                    }
                    _ => None,
                };
                Ok(artifacts)
            }
            _ => Err(format!("deployment artifacts must be an 'object' type")),
        }
    }

    pub fn from_bytes(bytes: &Vec<u8>) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompilationTool {
    Foundry(String),
    Hardhat,
}
