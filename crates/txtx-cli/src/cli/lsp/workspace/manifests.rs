//! Manifest parsing and management for the LSP workspace
//!
//! This module handles parsing and indexing of txtx.yml manifest files,
//! including tracking runbook references and environment configurations.

use lsp_types::Url;
use std::collections::HashMap;

/// Represents a parsed txtx manifest
#[derive(Debug, Clone)]
pub struct Manifest {
    pub uri: Url,
    pub runbooks: Vec<RunbookRef>,
    pub environments: HashMap<String, HashMap<String, String>>,
}

/// Reference to a runbook from a manifest
#[derive(Debug, Clone)]
pub struct RunbookRef {
    pub name: String,
    pub location: String,
    pub absolute_uri: Option<Url>,
}

impl Manifest {
    /// Parse a manifest from content
    pub fn parse(uri: Url, content: &str) -> Result<Self, String> {
        // Parse YAML content
        let yaml_value: serde_yml::Value =
            serde_yml::from_str(content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let yaml_mapping = yaml_value.as_mapping().ok_or("Expected YAML mapping at root")?;

        // Extract runbooks
        let mut runbooks = Vec::new();
        if let Some(runbooks_section) =
            yaml_mapping.get(&serde_yml::Value::String("runbooks".to_string()))
        {
            if let Some(runbooks_sequence) = runbooks_section.as_sequence() {
                for runbook_entry in runbooks_sequence {
                    if let Some(runbook_map) = runbook_entry.as_mapping() {
                        let name = runbook_map
                            .get(&serde_yml::Value::String("name".to_string()))
                            .and_then(|v| v.as_str())
                            .ok_or("Runbook missing 'name' field")?;
                        let location = runbook_map
                            .get(&serde_yml::Value::String("location".to_string()))
                            .and_then(|v| v.as_str())
                            .ok_or("Runbook missing 'location' field")?;

                        let absolute_uri = resolve_runbook_uri(&uri, location).ok();
                        runbooks.push(RunbookRef {
                            name: name.to_string(),
                            location: location.to_string(),
                            absolute_uri,
                        });
                    }
                }
            }
        }

        // Extract environments
        let mut environments = HashMap::new();
        if let Some(envs_section) =
            yaml_mapping.get(&serde_yml::Value::String("environments".to_string()))
        {
            if let Some(envs_map) = envs_section.as_mapping() {
                for (env_key, env_value) in envs_map {
                    if let Some(env_name) = env_key.as_str() {
                        if let Some(env_map) = env_value.as_mapping() {
                            let mut env_vars = HashMap::new();
                            for (key, value) in env_map {
                                if let Some(k) = key.as_str() {
                                    // Convert any YAML value to string representation
                                    let v = match value {
                                        serde_yml::Value::String(s) => s.clone(),
                                        serde_yml::Value::Number(n) => n.to_string(),
                                        serde_yml::Value::Bool(b) => b.to_string(),
                                        serde_yml::Value::Null => "null".to_string(),
                                        // For complex types, use debug representation or YAML serialization
                                        _ => serde_yml::to_string(value).unwrap_or_else(|_| format!("{:?}", value))
                                    };
                                    env_vars.insert(k.to_string(), v);
                                }
                            }
                            environments.insert(env_name.to_string(), env_vars);
                        }
                    }
                }
            }
        }

        Ok(Manifest { uri, runbooks, environments })
    }

    /// Find a runbook by name
    #[allow(dead_code)]
    pub fn find_runbook(&self, name: &str) -> Option<&RunbookRef> {
        self.runbooks.iter().find(|r| r.name == name)
    }

    /// Get environment variables for a specific environment
    #[allow(dead_code)]
    pub fn get_environment(&self, name: &str) -> Option<&HashMap<String, String>> {
        self.environments.get(name)
    }
}

/// Resolve a runbook location relative to a manifest URI
fn resolve_runbook_uri(manifest_uri: &Url, location: &str) -> Result<Url, String> {
    let manifest_path =
        manifest_uri.to_file_path().map_err(|_| "Failed to convert manifest URI to path")?;

    let manifest_dir = manifest_path.parent().ok_or("Manifest has no parent directory")?;

    let runbook_path = manifest_dir.join(location);

    Url::from_file_path(&runbook_path)
        .map_err(|_| format!("Failed to convert path to URI: {:?}", runbook_path))
}

/// Find the manifest file for a given runbook
pub fn find_manifest_for_runbook(runbook_uri: &Url) -> Option<Url> {
    let runbook_path = runbook_uri.to_file_path().ok()?;
    let mut current_dir = runbook_path.parent()?;

    // Walk up the directory tree looking for txtx.yml
    loop {
        // Check for various manifest file names
        let manifest_candidates = ["txtx.yml", "txtx.yaml", "Txtx.yml", "Txtx.yaml"];

        for candidate in &manifest_candidates {
            let manifest_path = current_dir.join(candidate);
            if manifest_path.exists() {
                return Url::from_file_path(&manifest_path).ok();
            }
        }

        current_dir = current_dir.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing() {
        let content = r#"
runbooks:
  - name: deploy
    location: runbooks/deploy.tx
  - name: test
    location: runbooks/test.tx

environments:
  prod:
    api_key: prod_key
    url: https://prod.example.com
  dev:
    api_key: dev_key
    url: https://dev.example.com
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        assert_eq!(manifest.runbooks.len(), 2);
        assert_eq!(manifest.environments.len(), 2);

        let deploy_runbook = manifest.find_runbook("deploy").unwrap();
        assert_eq!(deploy_runbook.location, "runbooks/deploy.tx");

        let prod_env = manifest.get_environment("prod").unwrap();
        assert_eq!(prod_env.get("api_key").unwrap(), "prod_key");
    }
}
