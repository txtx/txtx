//! Linter configuration

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use super::formatter::Format;
use super::rules::Severity;
use super::rule_id::CliRuleId;
use txtx_core::validation::CoreRuleId;
use serde::{Deserialize, Serialize};
use strum::{EnumString, Display, AsRefStr, IntoStaticStr};

/// Rule configuration from YAML file
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RuleConfig {
    /// Simple severity level (e.g., "error", "warning", "off")
    Severity(Severity),
    /// Full configuration with severity and options
    Full {
        severity: Severity,
        #[serde(default)]
        options: HashMap<String, serde_yaml::Value>,
    },
}

impl RuleConfig {
    /// Get the severity level for this rule
    pub fn severity(&self) -> Severity {
        match self {
            RuleConfig::Severity(s) => *s,
            RuleConfig::Full { severity, .. } => *severity,
        }
    }

    /// Check if rule is disabled
    pub fn is_disabled(&self) -> bool {
        self.severity() == Severity::Off
    }

    /// Get options for this rule
    pub fn options(&self) -> Option<&HashMap<String, serde_yaml::Value>> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Full { options, .. } => Some(options),
        }
    }
}

/// Configuration file structure (.txtxlint.yml)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ConfigFile {
    /// Base configuration to extend (e.g., "txtx:recommended")
    #[serde(rename = "extends", skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// Rule configurations mapped by rule ID
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,

    /// Paths to ignore during linting
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl ConfigFile {
    /// Load configuration from a YAML file
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML config: {}", e))
    }

    /// Load configuration from default locations
    pub fn load_default() -> Option<Self> {
        // Try .txtxlint.yml first, then .txtxlint.yaml
        for filename in &[".txtxlint.yml", ".txtxlint.yaml"] {
            let path = Path::new(filename);
            if path.exists() {
                if let Ok(config) = Self::from_file(path) {
                    return Some(config);
                }
            }
        }
        None
    }

    /// Load configuration from specified path or default locations
    pub fn load(config_path: Option<&str>) -> Option<Self> {
        if let Some(path) = config_path {
            Self::from_file(Path::new(path)).ok()
        } else {
            Self::load_default()
        }
    }

    /// Apply extends to get base configuration
    pub fn with_extends(&self) -> Self {
        let mut config = self.clone();

        if let Some(ref extends) = self.extends {
            if extends == "txtx:recommended" {
                // Apply recommended defaults
                let defaults = Self::recommended();

                // Merge rules (user config overrides defaults)
                for (rule, rule_config) in defaults.rules {
                    config.rules.entry(rule).or_insert(rule_config);
                }
            }
        }

        config
    }

    /// Get recommended configuration
    pub fn recommended() -> Self {
        let mut rules = HashMap::new();

        // Core validation rules with recommended severities (using CoreRuleId)
        rules.insert(
            CoreRuleId::UndefinedInput.as_ref().to_string(),
            RuleConfig::Severity(Severity::Error)
        );

        // CLI-specific rules (using CliRuleId)
        rules.insert(
            CliRuleId::CliInputOverride.as_ref().to_string(),
            RuleConfig::Severity(Severity::Info)
        );
        rules.insert(
            CliRuleId::InputNamingConvention.as_ref().to_string(),
            RuleConfig::Severity(Severity::Warning)
        );
        rules.insert(
            CliRuleId::NoSensitiveData.as_ref().to_string(),
            RuleConfig::Severity(Severity::Warning)
        );

        Self {
            extends: None,
            rules,
            ignore: vec![],
        }
    }

    /// Get rule configuration for a specific rule
    pub fn get_rule_config(&self, rule_id: &str) -> Option<&RuleConfig> {
        self.rules.get(rule_id)
    }

    /// Check if a rule is disabled
    pub fn is_rule_disabled(&self, rule_id: &str) -> bool {
        self.get_rule_config(rule_id)
            .map(|config| config.is_disabled())
            .unwrap_or(false)
    }

    /// Get severity for a rule (returns None if rule is disabled or not configured)
    pub fn get_rule_severity(&self, rule_id: &str) -> Option<Severity> {
        self.get_rule_config(rule_id)
            .and_then(|config| {
                let severity = config.severity();
                if severity == Severity::Off {
                    None
                } else {
                    Some(severity)
                }
            })
    }
}

#[derive(Clone, Debug)]
pub struct LinterConfig {
    pub manifest_path: Option<PathBuf>,
    pub runbook: Option<String>,
    pub environment: Option<String>,
    pub cli_inputs: Vec<(String, String)>,
    pub format: Format,
    pub config_file: Option<ConfigFile>,
}

impl LinterConfig {
    pub fn new(
        manifest_path: Option<PathBuf>,
        runbook: Option<String>,
        environment: Option<String>,
        cli_inputs: Vec<(String, String)>,
        format: Format,
    ) -> Self {
        Self {
            manifest_path,
            runbook,
            environment,
            cli_inputs,
            format,
            config_file: None,
        }
    }

    /// Create a new config with a loaded configuration file
    pub fn with_config_file(
        manifest_path: Option<PathBuf>,
        runbook: Option<String>,
        environment: Option<String>,
        cli_inputs: Vec<(String, String)>,
        format: Format,
        config_path: Option<&str>,
    ) -> Self {
        let config_file = ConfigFile::load(config_path)
            .map(|config| config.with_extends());

        Self {
            manifest_path,
            runbook,
            environment,
            cli_inputs,
            format,
            config_file,
        }
    }

    /// Check if a rule is disabled in the configuration
    pub fn is_rule_disabled(&self, rule_id: &str) -> bool {
        self.config_file
            .as_ref()
            .map(|config| config.is_rule_disabled(rule_id))
            .unwrap_or(false)
    }

    /// Get the configured severity for a rule
    pub fn get_rule_severity(&self, rule_id: &str) -> Option<Severity> {
        self.config_file
            .as_ref()
            .and_then(|config| config.get_rule_severity(rule_id))
    }
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            manifest_path: None,
            runbook: None,
            environment: None,
            cli_inputs: Vec::new(),
            format: Format::Stylish,
            config_file: None,
        }
    }
}