use alloy::json_abi::JsonAbi;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundryCompiledOutputJson {
    pub abi: JsonAbi,
    pub bytecode: ContractBytecode,
    pub deployed_bytecode: ContractBytecode,
    pub method_identifiers: JsonValue,
    pub raw_metadata: String,
    pub metadata: ContractMetadataJson,
    pub id: u16,
}

impl FoundryCompiledOutputJson {
    pub fn get_contract_source(
        &self,
        base_path: &str,
        contract_name: &str,
    ) -> Result<String, String> {
        let mut path = PathBuf::from_str(&base_path).unwrap();
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

        let source = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "invalid contract location {}: {}",
                path.to_str().unwrap_or(""),
                e
            )
        })?;
        Ok(source)
    }

    #[allow(dead_code)]
    pub async fn get_from_path(path: &str) -> Result<Self, String> {
        if path.starts_with("http") {
            todo!()
        } else {
            let artifact = std::fs::read(path)
                .map_err(|e| format!("invalid contract abi location {}: {}", path, e))?;

            let json: Self = serde_json::from_slice(&artifact)
                .map_err(|e| format!("invalid contract abi at location {}: {}", path, e))?;
            Ok(json)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBytecode {
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractMetadataJson {
    pub compiler: ContractCompilerVersion,
    pub language: String,
    pub output: JsonValue,
    pub settings: ContractSettings,
    pub sources: JsonValue,
    pub version: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSettings {
    pub compilation_target: HashMap<String, String>,
    pub optimizer: ContractOptimizerSettings,
    pub evm_version: String,
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
    pub src: String,
    pub out: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundryToml {
    pub profile: HashMap<String, FoundryProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundryConfig {
    pub toml: FoundryToml,
    pub toml_path: String,
}

impl FoundryConfig {
    pub fn get_compiled_output(
        &self,
        contract_filename: &str,
        contract_name: &str,
        profile_name: Option<&str>,
    ) -> Result<FoundryCompiledOutputJson, String> {
        let profile_name = profile_name.unwrap_or("default");
        let Some(profile) = self.toml.profile.get(profile_name) else {
            return Err(format!(
                "foundry.toml does not include profile {profile_name}",
            ));
        };
        let mut path = PathBuf::from_str(&self.toml_path).unwrap();
        path.pop();
        path.push(&format!("{}", profile.out));
        path.push(&format!("{}.sol", contract_filename));
        path.push(&format!("{}.json", contract_name));

        let bytes = std::fs::read(&path).map_err(|e| {
            format!(
                "invalid compiled output location {}: {}",
                &path.to_str().unwrap_or(""),
                e
            )
        })?;

        let config: FoundryCompiledOutputJson = serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "invalid foundry.toml at location {}: {}",
                &path.to_str().unwrap_or(""),
                e
            )
        })?;
        Ok(config)
    }

    pub fn get_from_path(foundry_toml_path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(foundry_toml_path)
            .map_err(|e| format!("invalid foundry.toml location {}: {}", foundry_toml_path, e))?;

        let toml: FoundryToml = toml::from_slice(&bytes).map_err(|e| {
            format!(
                "invalid foundry.toml at location {}: {}",
                foundry_toml_path, e
            )
        })?;

        let config = FoundryConfig {
            toml,
            toml_path: foundry_toml_path.to_string(),
        };
        Ok(config)
    }
}
