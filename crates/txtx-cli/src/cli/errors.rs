use error_stack::{Context, Report};
use std::fmt;
use txtx_addon_kit::types::errors::{TxtxError, ErrorAttachments};

/// CLI-specific error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliError {
    /// Manifest file not found or invalid
    ManifestError,
    /// Runbook not found in manifest
    RunbookNotFound,
    /// Failed to load configuration
    ConfigError,
    /// Failed to write output
    OutputError,
    /// Authentication failed
    AuthError,
    /// Network/service communication error
    ServiceError,
    /// Invalid command-line arguments
    ArgumentError,
    /// State file error
    StateError,
    /// Environment variable missing or invalid
    EnvironmentError,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::ManifestError => write!(f, "Manifest file error"),
            CliError::RunbookNotFound => write!(f, "Runbook not found"),
            CliError::ConfigError => write!(f, "Configuration error"),
            CliError::OutputError => write!(f, "Output operation failed"),
            CliError::AuthError => write!(f, "Authentication failed"),
            CliError::ServiceError => write!(f, "Service communication error"),
            CliError::ArgumentError => write!(f, "Invalid command arguments"),
            CliError::StateError => write!(f, "State file operation failed"),
            CliError::EnvironmentError => write!(f, "Environment configuration error"),
        }
    }
}

impl Context for CliError {}

/// Manifest-specific information
#[derive(Debug, Clone)]
pub struct ManifestInfo {
    pub path: String,
    pub expected_format: String,
}

impl fmt::Display for ManifestInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Manifest at '{}' (expected format: {})", self.path, self.expected_format)
    }
}

/// Runbook execution context
#[derive(Debug, Clone)]
pub struct RunbookContext {
    pub runbook_name: String,
    pub environment: Option<String>,
    pub manifest_path: String,
}

impl fmt::Display for RunbookContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Runbook '{}' from manifest '{}'", self.runbook_name, self.manifest_path)?;
        if let Some(env) = &self.environment {
            write!(f, " (environment: {})", env)?;
        }
        Ok(())
    }
}

/// Output operation details
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub destination: String,
    pub format: String,
    pub reason: String,
}

impl fmt::Display for OutputInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to write {} to '{}': {}", self.format, self.destination, self.reason)
    }
}

/// State file information
#[derive(Debug, Clone)]
pub struct StateFileInfo {
    pub path: String,
    pub operation: String,
}

impl fmt::Display for StateFileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State file '{}' (operation: {})", self.path, self.operation)
    }
}

/// Extension trait for CLI-specific error attachments
pub trait CliErrorExt {
    fn with_manifest_info(self, path: impl Into<String>, format: impl Into<String>) -> Self;
    fn with_runbook_context(self, name: impl Into<String>, manifest: impl Into<String>, env: Option<String>) -> Self;
    fn with_output_info(self, destination: impl Into<String>, format: impl Into<String>, reason: impl Into<String>) -> Self;
    fn with_state_file_info(self, path: impl Into<String>, operation: impl Into<String>) -> Self;
}

impl<T> CliErrorExt for Result<T, Report<CliError>> {
    fn with_manifest_info(self, path: impl Into<String>, format: impl Into<String>) -> Self {
        self.map_err(|e| e.attach(ManifestInfo {
            path: path.into(),
            expected_format: format.into(),
        }))
    }

    fn with_runbook_context(self, name: impl Into<String>, manifest: impl Into<String>, env: Option<String>) -> Self {
        self.map_err(|e| e.attach(RunbookContext {
            runbook_name: name.into(),
            environment: env,
            manifest_path: manifest.into(),
        }))
    }

    fn with_output_info(self, destination: impl Into<String>, format: impl Into<String>, reason: impl Into<String>) -> Self {
        self.map_err(|e| e.attach(OutputInfo {
            destination: destination.into(),
            format: format.into(),
            reason: reason.into(),
        }))
    }

    fn with_state_file_info(self, path: impl Into<String>, operation: impl Into<String>) -> Self {
        self.map_err(|e| e.attach(StateFileInfo {
            path: path.into(),
            operation: operation.into(),
        }))
    }
}

/// Helper for creating CLI errors
#[macro_export]
macro_rules! cli_error {
    ($error:expr, $($arg:tt)*) => {{
        use $crate::errors::CliError;
        error_stack::Report::new($error)
            .attach_printable(format!($($arg)*))
    }};
}

/// Convert string errors to CLI errors
pub trait IntoCliError<T> {
    fn into_cli_error(self, error_type: CliError) -> Result<T, Report<CliError>>;
}

impl<T> IntoCliError<T> for Result<T, String> {
    fn into_cli_error(self, error_type: CliError) -> Result<T, Report<CliError>> {
        self.map_err(|e| Report::new(error_type).attach_printable(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error_stack::ResultExt;

    #[test]
    fn test_manifest_error() {
        let error = Report::new(CliError::ManifestError)
            .attach_printable("Failed to parse Txtx.toml")
            .attach(ManifestInfo {
                path: "./Txtx.toml".to_string(),
                expected_format: "TOML".to_string(),
            });

        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Manifest file error"));
        assert!(error_string.contains("Failed to parse Txtx.toml"));
    }

    #[test]
    fn test_runbook_not_found() {
        fn find_runbook(name: &str) -> Result<(), Report<CliError>> {
            Err(Report::new(CliError::RunbookNotFound))
                .attach_printable(format!("No runbook named '{}'", name))
                .with_runbook_context(name, "./Txtx.toml", Some("production".to_string()))
        }

        let error = find_runbook("deploy").unwrap_err();
        let context = error.downcast_ref::<RunbookContext>().unwrap();
        assert_eq!(context.runbook_name, "deploy");
        assert_eq!(context.environment, Some("production".to_string()));
    }

    #[test]
    fn test_string_error_conversion() {
        fn load_manifest() -> Result<String, String> {
            Err("File not found: Txtx.toml".to_string())
        }

        let result = load_manifest().into_cli_error(CliError::ManifestError);
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("File not found: Txtx.toml"));
    }
}