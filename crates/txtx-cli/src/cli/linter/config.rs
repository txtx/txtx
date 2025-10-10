//! Linter configuration

use std::path::PathBuf;
use super::formatter::Format;

#[derive(Clone, Debug)]
pub struct LinterConfig {
    pub manifest_path: Option<PathBuf>,
    pub runbook: Option<String>,
    pub environment: Option<String>,
    pub cli_inputs: Vec<(String, String)>,
    pub format: Format,
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
        }
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
        }
    }
}