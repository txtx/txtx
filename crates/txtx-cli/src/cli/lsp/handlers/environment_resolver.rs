//! Environment resolution utilities for LSP
//!
//! Provides utilities for resolving values across different environments
//! with proper inheritance from global environment.

use crate::cli::lsp::workspace::Manifest;
use lsp_types::Url;
use std::collections::HashMap;

pub struct EnvironmentResolver<'a> {
    manifest: &'a Manifest,
    current_env: String,
}

impl<'a> EnvironmentResolver<'a> {
    pub fn new(manifest: &'a Manifest, current_env: String) -> Self {
        Self {
            manifest,
            current_env,
        }
    }

    /// Resolve a value for a key in the current environment, with inheritance from global
    /// Returns (value, source_environment)
    pub fn resolve_value(&self, key: &str) -> Option<(String, String)> {
        // First check the current environment
        if let Some(env_vars) = self.manifest.environments.get(&self.current_env) {
            if let Some(value) = env_vars.get(key) {
                return Some((value.clone(), self.current_env.clone()));
            }
        }

        // If not found and we're not in global, check global environment
        if self.current_env != "global" {
            if let Some(global_vars) = self.manifest.environments.get("global") {
                if let Some(value) = global_vars.get(key) {
                    return Some((value.clone(), "global".to_string()));
                }
            }
        }

        None
    }

    /// Get all values for a key across all environments
    /// Returns Vec of (environment_name, value)
    pub fn get_all_values(&self, key: &str) -> Vec<(String, String)> {
        let mut values = Vec::new();

        for (env_name, env_vars) in &self.manifest.environments {
            if let Some(value) = env_vars.get(key) {
                values.push((env_name.clone(), value.clone()));
            }
        }

        // Sort by environment name for consistent output
        values.sort_by(|a, b| a.0.cmp(&b.0));
        values
    }

    /// Get all effective inputs for the current environment
    /// Returns HashMap of key -> (value, source_environment)
    pub fn get_effective_inputs(&self) -> HashMap<String, (String, String)> {
        let mut effective_inputs = HashMap::new();

        // First add global inputs (lowest precedence)
        if let Some(global_vars) = self.manifest.environments.get("global") {
            for (key, value) in global_vars {
                effective_inputs.insert(key.clone(), (value.clone(), "global".to_string()));
            }
        }

        // Then override with environment-specific inputs (higher precedence)
        if self.current_env != "global" {
            if let Some(env_vars) = self.manifest.environments.get(&self.current_env) {
                for (key, value) in env_vars {
                    effective_inputs.insert(key.clone(), (value.clone(), self.current_env.clone()));
                }
            }
        }

        effective_inputs
    }

    /// Check if a value is inherited from global environment
    pub fn is_inherited_from_global(&self, key: &str) -> bool {
        if self.current_env == "global" {
            return false;
        }

        // Check if it exists in current environment
        let exists_in_current = self.manifest.environments
            .get(&self.current_env)
            .and_then(|vars| vars.get(key))
            .is_some();

        // Check if it exists in global
        let exists_in_global = self.manifest.environments
            .get("global")
            .and_then(|vars| vars.get(key))
            .is_some();

        !exists_in_current && exists_in_global
    }

    /// Get all environment names sorted
    pub fn get_all_environments(&self) -> Vec<String> {
        let mut env_names: Vec<_> = self.manifest.environments.keys().cloned().collect();
        env_names.sort();
        env_names
    }

    /// Count how many environments override a specific value from global
    pub fn count_overrides(&self, key: &str) -> usize {
        let global_value = self.manifest.environments
            .get("global")
            .and_then(|vars| vars.get(key));

        if global_value.is_none() {
            return 0;
        }

        self.manifest.environments
            .iter()
            .filter(|(name, vars)| {
                name != &"global" && 
                vars.get(key).is_some() && 
                vars.get(key) != global_value
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> Manifest {
        let mut manifest = Manifest {
            uri: Url::parse("file:///test/txtx.yml").unwrap(),
            runbooks: Vec::new(),
            environments: HashMap::new(),
        };

        // Add global environment
        let mut global_vars = HashMap::new();
        global_vars.insert("api_key".to_string(), "global_key".to_string());
        global_vars.insert("url".to_string(), "https://global.com".to_string());
        manifest.environments.insert("global".to_string(), global_vars);

        // Add dev environment
        let mut dev_vars = HashMap::new();
        dev_vars.insert("api_key".to_string(), "dev_key".to_string());
        dev_vars.insert("dev_only".to_string(), "dev_value".to_string());
        manifest.environments.insert("dev".to_string(), dev_vars);

        // Add prod environment
        let mut prod_vars = HashMap::new();
        prod_vars.insert("api_key".to_string(), "prod_key".to_string());
        manifest.environments.insert("prod".to_string(), prod_vars);

        manifest
    }

    #[test]
    fn test_resolve_value() {
        let manifest = create_test_manifest();
        let resolver = EnvironmentResolver::new(&manifest, "dev".to_string());

        // Test value from current environment
        let result = resolver.resolve_value("api_key");
        assert_eq!(result, Some(("dev_key".to_string(), "dev".to_string())));

        // Test value only in current environment
        let result = resolver.resolve_value("dev_only");
        assert_eq!(result, Some(("dev_value".to_string(), "dev".to_string())));

        // Test value inherited from global
        let result = resolver.resolve_value("url");
        assert_eq!(result, Some(("https://global.com".to_string(), "global".to_string())));

        // Test non-existent value
        let result = resolver.resolve_value("missing");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_all_values() {
        let manifest = create_test_manifest();
        let resolver = EnvironmentResolver::new(&manifest, "dev".to_string());

        let values = resolver.get_all_values("api_key");
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], ("dev".to_string(), "dev_key".to_string()));
        assert_eq!(values[1], ("global".to_string(), "global_key".to_string()));
        assert_eq!(values[2], ("prod".to_string(), "prod_key".to_string()));
    }

    #[test]
    fn test_get_effective_inputs() {
        let manifest = create_test_manifest();
        let resolver = EnvironmentResolver::new(&manifest, "dev".to_string());

        let inputs = resolver.get_effective_inputs();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs.get("api_key"), Some(&("dev_key".to_string(), "dev".to_string())));
        assert_eq!(inputs.get("url"), Some(&("https://global.com".to_string(), "global".to_string())));
        assert_eq!(inputs.get("dev_only"), Some(&("dev_value".to_string(), "dev".to_string())));
    }

    #[test]
    fn test_is_inherited_from_global() {
        let manifest = create_test_manifest();
        let resolver = EnvironmentResolver::new(&manifest, "dev".to_string());

        assert!(!resolver.is_inherited_from_global("api_key")); // Overridden in dev
        assert!(resolver.is_inherited_from_global("url")); // Only in global
        assert!(!resolver.is_inherited_from_global("dev_only")); // Only in dev
        assert!(!resolver.is_inherited_from_global("missing")); // Doesn't exist
    }

    #[test]
    fn test_count_overrides() {
        let manifest = create_test_manifest();
        let resolver = EnvironmentResolver::new(&manifest, "dev".to_string());

        assert_eq!(resolver.count_overrides("api_key"), 2); // Overridden in dev and prod
        assert_eq!(resolver.count_overrides("url"), 0); // Not overridden
        assert_eq!(resolver.count_overrides("missing"), 0); // Doesn't exist in global
    }
}