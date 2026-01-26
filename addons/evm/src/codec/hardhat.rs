use alloy_json_abi::JsonAbi;
use std::{collections::HashMap, path::PathBuf};

const BUILD_INFO_DIR: &str = "build-info";

#[derive(Clone, Debug)]
pub struct HardhatBuildArtifacts {
    pub compiled_contract_path: String,
    pub artifacts: HardhatContractArtifacts,
    pub build_info: HardhatContractBuildInfo,
}

impl HardhatBuildArtifacts {
    pub fn new(
        artifacts_path: PathBuf,
        contract_source_path: &str,
        contract_name: &str,
    ) -> Result<Self, String> {
        let build_info =
            HardhatContractBuildInfo::new(&artifacts_path, contract_source_path, contract_name)?;

        let mut compiled_contract_path = artifacts_path;
        compiled_contract_path.push(contract_source_path);

        let contract_artifacts =
            HardhatContractArtifacts::new(compiled_contract_path.clone(), contract_name)?;

        Ok(Self {
            compiled_contract_path: compiled_contract_path.to_str().unwrap().to_string(),
            artifacts: contract_artifacts,
            build_info,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// The <hash>.json file found in <root>/<artifacts-dir>/build-info folder of a hardhat project
pub struct HardhatContractBuildInfo {
    pub solc_long_version: String,
    pub input: HardhatContractBuildInfoInput,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardhatContractBuildInfoInput {
    pub sources: HashMap<String, HardhatContractBuildInfoSource>,
    pub settings: HardhatContractSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardhatContractBuildInfoSource {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardhatContractSettings {
    pub evm_version: String,
    pub optimizer: HardhatContractOptimizerSettings,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardhatContractOptimizerSettings {
    pub enabled: bool,
    pub runs: u64,
}

impl HardhatContractBuildInfo {
    pub fn new(
        artifacts_path: &PathBuf,
        contract_source_path: &str,
        contract_name: &str,
    ) -> Result<Self, String> {
        let build_info_file_hash: String = HardhatContractDebugFile::get_dbg_file_hash(
            &artifacts_path,
            contract_source_path,
            contract_name,
        )?;

        let mut contract_build_info_path = artifacts_path.clone();
        contract_build_info_path.push(&BUILD_INFO_DIR);
        contract_build_info_path.push(&build_info_file_hash);

        let bytes = std::fs::read(&contract_build_info_path).map_err(|e| {
            format!(
                "invalid hardhat build info location {}: {}",
                &contract_build_info_path.to_str().unwrap_or(""),
                e
            )
        })?;

        let build_info: HardhatContractBuildInfo = serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "invalid hardhat build info at location {}: {}",
                &contract_build_info_path.to_str().unwrap_or(""),
                e
            )
        })?;

        Ok(build_info)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// The <root>/<sources-dir>/<sol-filename>/<contract_name>.json file containing the compiled contract artifacts
pub struct HardhatContractArtifacts {
    pub abi: JsonAbi,
    pub bytecode: String,
    pub deployed_bytecode: String,
    pub source_name: String,
    pub contract_name: String,
}

impl HardhatContractArtifacts {
    pub fn new(mut compiled_contract_path: PathBuf, contract_name: &str) -> Result<Self, String> {
        compiled_contract_path.push(&format!("{}.json", contract_name));

        let bytes = std::fs::read(&compiled_contract_path).map_err(|e| {
            format!(
                "invalid hardhat artifacts location {}: {}",
                &compiled_contract_path.to_str().unwrap_or(""),
                e
            )
        })?;

        let artifacts: HardhatContractArtifacts = serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "invalid hardhat artifacts at location {}: {}",
                &compiled_contract_path.to_str().unwrap_or(""),
                e
            )
        })?;

        Ok(artifacts)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// The <root>/<sources-dir>/<sol-filename>/<contract_name>.dbg.json file containing the location of the contract's build info.
pub struct HardhatContractDebugFile {
    build_info: String,
}

impl HardhatContractDebugFile {
    pub fn get_dbg_file_hash(
        artifacts_path: &PathBuf,
        contract_source_path: &str,
        contract_name: &str,
    ) -> Result<String, String> {
        let mut contract_db_json_path = artifacts_path.clone();
        contract_db_json_path.push(contract_source_path);
        contract_db_json_path.push(&format!("{}.dbg.json", contract_name));

        let bytes = std::fs::read(&contract_db_json_path).map_err(|e| {
            format!(
                "invalid hardhat debug artifacts location {}: {}",
                &contract_db_json_path.to_str().unwrap_or(""),
                e
            )
        })?;

        let dbg: HardhatContractDebugFile = serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "invalid hardhat debug artifacts at location {}: {}",
                &contract_db_json_path.to_str().unwrap_or(""),
                e
            )
        })?;
        let build_info_parts: Vec<&str> = dbg.build_info.split("/").collect();
        let hash = build_info_parts.last().ok_or(format!(
            "could not find hardhat build info for contract {}.sol/{}",
            contract_db_json_path.to_str().unwrap(),
            contract_name
        ))?;

        Ok(hash.to_string())
    }
}
