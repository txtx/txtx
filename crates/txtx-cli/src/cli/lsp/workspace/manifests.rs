//! txtx.yml manifest parsing and indexing

use lsp_types::Url;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Represents a parsed txtx manifest
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    #[serde(skip, default = "default_url")]
    pub uri: Url,

    #[serde(default)]
    pub runbooks: Vec<RunbookRef>,

    #[serde(default, deserialize_with = "deserialize_environments")]
    pub environments: HashMap<String, HashMap<String, String>>,
}

/// Default URL for when deserializing without a uri
fn default_url() -> Url {
    Url::parse("file:///").expect("Failed to parse default URL")
}

/// Custom deserializer for environments that converts all values to strings
fn deserialize_environments<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, HashMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, HashMap<String, serde_yml::Value>> =
        HashMap::deserialize(deserializer)?;

    let mut result = HashMap::new();
    for (env_name, env_vars) in raw {
        let mut string_vars = HashMap::new();
        for (key, value) in env_vars {
            let string_value = match value {
                serde_yml::Value::String(s) => s,
                serde_yml::Value::Number(n) => n.to_string(),
                serde_yml::Value::Bool(b) => b.to_string(),
                serde_yml::Value::Null => "null".to_string(),
                _ => serde_yml::to_string(&value)
                    .unwrap_or_else(|_| format!("{:?}", value)),
            };
            string_vars.insert(key, string_value);
        }
        result.insert(env_name, string_vars);
    }

    Ok(result)
}

/// Reference to a runbook from a manifest
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunbookRef {
    pub name: String,
    pub location: String,

    #[serde(skip, default)]
    pub absolute_uri: Option<Url>,
}

impl Manifest {
    /// Parse a manifest from content
    pub fn parse(uri: Url, content: &str) -> Result<Self, String> {
        // Parse using Serde
        let mut manifest: Self =
            serde_yml::from_str(content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        // Set the URI (skipped during deserialization)
        manifest.uri = uri.clone();

        // Resolve absolute URIs for runbooks
        for runbook in &mut manifest.runbooks {
            runbook.absolute_uri = resolve_runbook_uri(&uri, &runbook.location).ok();
        }

        Ok(manifest)
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
    fn test_manifest_parsing_basic() {
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

        // Test direct field access (how LSP actually uses it)
        let deploy = manifest.runbooks.iter().find(|r| r.name == "deploy").unwrap();
        assert_eq!(deploy.location, "runbooks/deploy.tx");

        let prod_env = manifest.environments.get("prod").unwrap();
        assert_eq!(prod_env.get("api_key").unwrap(), "prod_key");
    }

    #[test]
    fn test_global_environment_handling() {
        let content = r#"
environments:
  global:
    api_key: global_key
    timeout: "30"
  prod:
    api_key: prod_key
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        // Test global environment exists
        let global = manifest.environments.get("global").unwrap();
        assert_eq!(global.get("api_key").unwrap(), "global_key");
        assert_eq!(global.get("timeout").unwrap(), "30");

        // Test environment inheritance pattern (global as fallback)
        let prod = manifest.environments.get("prod").unwrap();
        assert_eq!(prod.get("api_key").unwrap(), "prod_key");
        assert!(prod.get("timeout").is_none()); // Not in prod, would fall back to global

        // Verify global fallback pattern works
        let timeout = prod.get("timeout").or_else(|| global.get("timeout"));
        assert_eq!(timeout.unwrap(), "30");
    }

    #[test]
    fn test_empty_sections() {
        let content = r#"
runbooks: []
environments: {}
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        assert_eq!(manifest.runbooks.len(), 0);
        assert_eq!(manifest.environments.len(), 0);
    }

    #[test]
    fn test_missing_sections() {
        // Empty object is valid, but sections are optional
        let content = r#"{}"#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        // Should not fail, just return empty collections
        assert_eq!(manifest.runbooks.len(), 0);
        assert_eq!(manifest.environments.len(), 0);
    }

    #[test]
    fn test_only_runbooks_section() {
        let content = r#"
runbooks:
  - name: deploy
    location: deploy.tx
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        assert_eq!(manifest.runbooks.len(), 1);
        assert_eq!(manifest.environments.len(), 0);
    }

    #[test]
    fn test_only_environments_section() {
        let content = r#"
environments:
  dev:
    api_key: dev_key
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        assert_eq!(manifest.runbooks.len(), 0);
        assert_eq!(manifest.environments.len(), 1);
    }

    #[test]
    fn test_parse_error_invalid_yaml() {
        let content = r#"
runbooks:
  - name: deploy
    location: deploy.tx
    invalid_indent:
  wrong: structure
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let result = Manifest::parse(uri, content);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Failed to parse YAML") || error.contains("YAML"));
    }

    #[test]
    fn test_parse_error_missing_required_fields() {
        let content = r#"
runbooks:
  - location: deploy.tx
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let result = Manifest::parse(uri, content);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("name"));
    }

    #[test]
    fn test_environment_value_types() {
        let content = r#"
environments:
  test:
    string_val: "hello"
    number_val: 42
    bool_val: true
    null_val: null
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        let test_env = manifest.environments.get("test").unwrap();
        assert_eq!(test_env.get("string_val").unwrap(), "hello");
        assert_eq!(test_env.get("number_val").unwrap(), "42");
        assert_eq!(test_env.get("bool_val").unwrap(), "true");
        assert_eq!(test_env.get("null_val").unwrap(), "null");
    }

    #[test]
    fn test_environment_keys_iteration() {
        let content = r#"
environments:
  global:
    key1: val1
  dev:
    key2: val2
  prod:
    key3: val3
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        // Test key iteration (used for completions in LSP)
        let mut env_names: Vec<_> = manifest.environments.keys().cloned().collect();
        env_names.sort();

        assert_eq!(env_names, vec!["dev", "global", "prod"]);
    }

    #[test]
    fn test_runbook_iteration_pattern() {
        let content = r#"
runbooks:
  - name: deploy
    location: deploy.tx
  - name: test
    location: test.tx
  - name: build
    location: build.tx
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        // Test iteration pattern used in LSP
        let runbook_names: Vec<_> = manifest.runbooks.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(runbook_names, vec!["deploy", "test", "build"]);

        // Test find pattern used in LSP
        let found = manifest.runbooks.iter().find(|r| r.name == "test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().location, "test.tx");
    }

    #[test]
    fn test_runbook_absolute_uri_resolution() {
        let content = r#"
runbooks:
  - name: deploy
    location: runbooks/deploy.tx
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        let deploy = &manifest.runbooks[0];
        assert!(deploy.absolute_uri.is_some());

        let absolute = deploy.absolute_uri.as_ref().unwrap();
        assert!(absolute.as_str().contains("runbooks/deploy.tx"));
    }

    #[test]
    fn test_manifest_uri_preserved() {
        let content = r#"
runbooks: []
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri.clone(), content).unwrap();

        assert_eq!(manifest.uri, uri);
    }

    #[test]
    fn test_environment_direct_access_pattern() {
        let content = r#"
environments:
  global:
    base_url: https://api.example.com
    timeout: "30"
  prod:
    api_key: prod_key
        "#;

        let uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest = Manifest::parse(uri, content).unwrap();

        // Pattern used in environment_resolver.rs
        let current_env = "prod";

        // Check current environment
        let env_vars = manifest.environments.get(current_env);
        assert!(env_vars.is_some());
        assert!(env_vars.unwrap().get("api_key").is_some());

        // Check global fallback
        let global_vars = manifest.environments.get("global");
        assert!(global_vars.is_some());
        assert_eq!(global_vars.unwrap().get("base_url").unwrap(), "https://api.example.com");
    }
}
