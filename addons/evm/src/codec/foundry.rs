use alloy::json_abi::JsonAbi;
use foundry_compilers_artifacts_solc::Metadata;
use foundry_config::figment::{
    providers::{Format, Toml},
    Figment,
};
pub use foundry_config::Config as FoundryConfig;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use txtx_addon_kit::helpers::fs::FileLocation;

use crate::constants::DEFAULT_FOUNDRY_OUT_DIR;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundryCompiledOutputJson {
    pub abi: JsonAbi,
    pub bytecode: ContractBytecode,
    pub deployed_bytecode: ContractBytecode,
    pub method_identifiers: JsonValue,
    pub raw_metadata: String,
    pub metadata: Metadata,
    pub id: u16,
}

impl FoundryCompiledOutputJson {
    pub fn get_contract_path(
        &self,
        base_path: &FileLocation,
        contract_name: &str,
    ) -> Result<PathBuf, String> {
        let mut path = PathBuf::from(&base_path.expect_path_buf());
        path.pop();
        let Some(contract_path) = self
            .metadata
            .settings
            .compilation_target
            .iter()
            .find(|(_, target)| target.eq(&contract_name))
            .map(|(path, _)| path)
        else {
            return Err(format!("could not find compilation target {contract_name}"));
        };
        path.push(&contract_path);
        Ok(path)
    }

    pub fn get_contract_source(
        &self,
        base_path: &FileLocation,
        contract_name: &str,
    ) -> Result<String, String> {
        let path = self.get_contract_path(base_path, contract_name)?;
        let source = std::fs::read_to_string(&path).map_err(|e| {
            format!("invalid contract location {}: {}", path.to_str().unwrap_or(""), e)
        })?;
        Ok(source)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBytecode {
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSettings {
    pub compilation_target: HashMap<String, String>,
    pub optimizer: ContractOptimizerSettings,
    pub evm_version: String,
    pub remappings: Vec<String>,
    #[serde(rename = "viaIR")]
    pub via_ir: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractOptimizerSettings {
    pub enabled: bool,
    pub runs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractCompilerVersion {
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundryProfile {
    pub src: Option<String>,
    pub out: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FoundryToml {
    pub figment: Figment,
    pub toml_path: String,
}

impl FoundryToml {
    pub fn get_foundry_config(&self, profile_name: Option<&str>) -> Result<FoundryConfig, String> {
        let profile_name = profile_name.unwrap_or("default");
        let figment = self.figment.clone();

        let foundry_config: FoundryConfig = figment
            .select(profile_name)
            .extract()
            .map_err(|e| format!("foundry.toml does not include profile {profile_name}: {}", e))?;
        Ok(foundry_config)
    }

    pub fn get_compiled_output(
        &self,
        contract_name: &str,
        contract_filename: &str,
        profile_name: Option<&str>,
    ) -> Result<FoundryCompiledOutputJson, String> {
        let foundry_config = self.get_foundry_config(profile_name)?;

        let mut path = PathBuf::from_str(&self.toml_path).unwrap();
        path.pop();
        path.push(&format!("{}", foundry_config.out.to_str().unwrap_or(DEFAULT_FOUNDRY_OUT_DIR)));
        path.push(contract_filename);
        path.push(&format!("{}.json", contract_name));

        let bytes = std::fs::read(&path).map_err(|e| {
            format!("invalid compiled output location {}: {}", &path.to_str().unwrap_or(""), e)
        })?;

        let config: FoundryCompiledOutputJson = serde_json::from_slice(&bytes).map_err(|e| {
            format!("invalid compiled output at location {}: {}", &path.to_str().unwrap_or(""), e)
        })?;
        Ok(config)
    }

    pub fn new(foundry_toml_path: &FileLocation) -> Result<Self, String> {
        let figment =
            FoundryConfig::figment().merge(Toml::file_exact(foundry_toml_path.expect_path_buf()));

        let config = FoundryToml { figment, toml_path: foundry_toml_path.to_string() };
        Ok(config)
    }
}
