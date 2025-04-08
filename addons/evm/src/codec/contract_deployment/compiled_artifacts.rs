use std::{path::PathBuf, str::FromStr};

use alloy::json_abi::JsonAbi;
use foundry_compilers_artifacts_solc::Metadata;
use foundry_config::{ethers_solc::Project, Config};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::typing::EvmValue;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompiledContractArtifacts {
    pub bytecode: String,
    pub abi: Option<JsonAbi>,
    pub source: Option<String>,
    pub contract_name: Option<String>,
    pub contract_filename: Option<String>,
    /// Will usually be ContractName.sol/ContractName, but in cases where there are multiple
    /// contracts per .sol file, it could be ContractName.sol/AnotherContractName
    pub contract_target_path: Option<String>,
    pub compilation_tool: Option<CompilationTool>,
    pub foundry_config: Option<Config>,
    pub metadata: Option<Metadata>,
}

impl CompiledContractArtifacts {
    pub fn new(bytecode: &str) -> Self {
        let mut default = CompiledContractArtifacts::default();
        default.bytecode = bytecode.to_string();
        default
    }

    pub fn contract_target_path_buf(&self) -> Result<PathBuf, Diagnostic> {
        let contract_target_path = self.contract_target_path.as_ref().ok_or_else(|| {
            diagnosed_error!("missing expected contract target path in compiled contract artifacts")
        })?;
        PathBuf::from_str(contract_target_path)
            .map_err(|e| diagnosed_error!("failed to parse contract target path: {}", e))
    }

    pub fn project(&self) -> Result<Project, Diagnostic> {
        self.foundry_config
            .as_ref()
            .ok_or_else(|| {
                diagnosed_error!("missing expected foundry config in compiled contract artifacts")
            })?
            .project()
            .map_err(|e| diagnosed_error!("failed to load foundry project from config: {}", e))
    }

    pub fn from_map(object: &IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let Some(bytecode) = object.get("bytecode") else {
            return Err(diagnosed_error!(
                "compiled contract artifacts missing required 'bytecode' key"
            ));
        };
        let mut artifacts = CompiledContractArtifacts::new(&bytecode.expect_string());

        artifacts.abi = object
            .get("abi")
            .map(|v| {
                v.as_string()
                    .map(|abi_str| {
                        serde_json::from_str::<JsonAbi>(&abi_str)
                            .map_err(|e| diagnosed_error!("failed to decode contract abi: {e}"))
                    })
                    .ok_or_else(|| diagnosed_error!("expected abi to be a string"))?
            })
            .transpose()?;

        artifacts.metadata = object
            .get("metadata")
            .map(|v| EvmValue::to_foundry_compiled_metadata(v))
            .transpose()?;

        artifacts.source =
            object.get("source").and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));

        artifacts.contract_name = object
            .get("contract_name")
            .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
        artifacts.contract_filename = object
            .get("contract_filename")
            .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));
        artifacts.contract_target_path = object
            .get("contract_target_path")
            .and_then(|v| v.as_string().and_then(|s| Some(s.to_string())));

        artifacts.foundry_config = match object.get("foundry_config") {
            Some(Value::Buffer(config)) => {
                let config = serde_json::from_slice(&config)
                    .map_err(|e| diagnosed_error!("invalid foundry config: {}", e))?;
                Some(config)
            }
            _ => None,
        };

        Ok(artifacts)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompilationTool {
    Foundry(String),
    Hardhat,
}
