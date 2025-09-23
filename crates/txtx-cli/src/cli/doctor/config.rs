use std::path::PathBuf;

/// Configuration for the doctor command
#[derive(Debug)]
pub struct DoctorConfig {
    pub manifest_path: PathBuf,
    pub runbook_name: Option<String>,
    pub environment: Option<String>,
    pub cli_inputs: Vec<(String, String)>,
    pub format: crate::cli::DoctorOutputFormat,
}

impl DoctorConfig {
    /// Create a new doctor configuration
    pub fn new(
        manifest_path: Option<String>,
        runbook_name: Option<String>,
        environment: Option<String>,
        cli_inputs: Vec<(String, String)>,
        format: crate::cli::DoctorOutputFormat,
    ) -> Self {
        let manifest_path =
            PathBuf::from(manifest_path.unwrap_or_else(|| "./txtx.yml".to_string()));
        Self { manifest_path, runbook_name, environment, cli_inputs, format }
    }

    /// Resolve auto format to concrete format
    pub fn resolve_format(mut self) -> Self {
        self.format = match self.format {
            crate::cli::DoctorOutputFormat::Auto => detect_output_format(),
            other => other,
        };
        self
    }

    /// Check if we should print diagnostics
    pub fn should_print_diagnostics(&self) -> bool {
        matches!(self.format, crate::cli::DoctorOutputFormat::Pretty)
    }
}

/// Auto-detect the appropriate output format based on environment
fn detect_output_format() -> crate::cli::DoctorOutputFormat {
    // Check environment variable first
    if let Ok(env_format) = std::env::var("TXTX_DOCTOR_FORMAT") {
        match env_format.to_lowercase().as_str() {
            "quickfix" => return crate::cli::DoctorOutputFormat::Quickfix,
            "json" => return crate::cli::DoctorOutputFormat::Json,
            "pretty" => return crate::cli::DoctorOutputFormat::Pretty,
            _ => {} // Fall through to auto-detection
        }
    }

    // Check if output is being piped or we're in CI
    if !atty::is(atty::Stream::Stdout) || std::env::var("CI").is_ok() {
        crate::cli::DoctorOutputFormat::Quickfix
    } else {
        crate::cli::DoctorOutputFormat::Pretty
    }
}
