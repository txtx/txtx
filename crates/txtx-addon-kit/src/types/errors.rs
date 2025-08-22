use error_stack::{Context, Report, ResultExt};
use std::fmt;

/// Core error types for txtx operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxtxError {
    /// Errors during parsing phase
    Parsing,
    /// Errors during validation phase
    Validation,
    /// Errors during execution phase
    Execution,
    /// Type system errors
    TypeMismatch,
    /// Missing required inputs
    MissingInput,
    /// Network communication errors
    Network,
    /// Signer-related errors
    Signer,
}

impl fmt::Display for TxtxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxtxError::Parsing => write!(f, "Failed to parse runbook"),
            TxtxError::Validation => write!(f, "Validation failed"),
            TxtxError::Execution => write!(f, "Execution failed"),
            TxtxError::TypeMismatch => write!(f, "Type mismatch"),
            TxtxError::MissingInput => write!(f, "Missing required input"),
            TxtxError::Network => write!(f, "Network operation failed"),
            TxtxError::Signer => write!(f, "Signer operation failed"),
        }
    }
}

impl Context for TxtxError {}

/// Attachments for error context
#[derive(Debug, Clone)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at {}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct ErrorDocumentation {
    pub help: String,
    pub example: Option<String>,
    pub link: Option<String>,
}

impl fmt::Display for ErrorDocumentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Help: {}", self.help)?;
        if let Some(example) = &self.example {
            write!(f, "\nExample: {}", example)?;
        }
        if let Some(link) = &self.link {
            write!(f, "\nSee: {}", link)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ActionContext {
    pub action_name: String,
    pub namespace: String,
    pub construct_id: String,
}

impl fmt::Display for ActionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in action '{}' ({}::{})", self.action_name, self.namespace, self.construct_id)
    }
}

#[derive(Debug, Clone)]
pub struct TypeMismatchInfo {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

impl fmt::Display for TypeMismatchInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "field '{}' expected {} but got {}", self.field, self.expected, self.actual)
    }
}

/// Extension trait for adding common attachments
pub trait ErrorAttachments<T> {
    fn with_location(self, file: &str, line: u32, column: u32) -> Self;
    fn with_documentation(self, help: impl Into<String>) -> Self;
    fn with_example(self, example: impl Into<String>) -> Self;
    fn with_link(self, link: impl Into<String>) -> Self;
    fn with_action_context(
        self,
        action_name: impl Into<String>,
        namespace: impl Into<String>,
        construct_id: impl Into<String>,
    ) -> Self;
}

impl<T, C> ErrorAttachments<T> for Result<T, Report<C>>
where
    C: Context,
{
    fn with_location(self, file: &str, line: u32, column: u32) -> Self {
        self.attach(ErrorLocation { file: file.to_string(), line, column })
    }

    fn with_documentation(self, help: impl Into<String>) -> Self {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                let docs = match err.downcast_ref::<ErrorDocumentation>() {
                    Some(existing) => ErrorDocumentation {
                        help: help.into(),
                        example: existing.example.clone(),
                        link: existing.link.clone(),
                    },
                    None => ErrorDocumentation { help: help.into(), example: None, link: None },
                };
                Err(err.attach(docs))
            }
        }
    }

    fn with_example(self, example: impl Into<String>) -> Self {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                let docs = match err.downcast_ref::<ErrorDocumentation>() {
                    Some(existing) => ErrorDocumentation {
                        help: existing.help.clone(),
                        example: Some(example.into()),
                        link: existing.link.clone(),
                    },
                    None => ErrorDocumentation {
                        help: String::new(),
                        example: Some(example.into()),
                        link: None,
                    },
                };
                Err(err.attach(docs))
            }
        }
    }

    fn with_link(self, link: impl Into<String>) -> Self {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                let docs = match err.downcast_ref::<ErrorDocumentation>() {
                    Some(existing) => ErrorDocumentation {
                        help: existing.help.clone(),
                        example: existing.example.clone(),
                        link: Some(link.into()),
                    },
                    None => ErrorDocumentation {
                        help: String::new(),
                        example: None,
                        link: Some(link.into()),
                    },
                };
                Err(err.attach(docs))
            }
        }
    }

    fn with_action_context(
        self,
        action_name: impl Into<String>,
        namespace: impl Into<String>,
        construct_id: impl Into<String>,
    ) -> Self {
        self.attach(ActionContext {
            action_name: action_name.into(),
            namespace: namespace.into(),
            construct_id: construct_id.into(),
        })
    }
}

/// Compatibility layer for migrating from Diagnostic
use crate::types::diagnostics::{Diagnostic, DiagnosticLevel};

impl From<Diagnostic> for Report<TxtxError> {
    fn from(diag: Diagnostic) -> Self {
        let base_error = match diag.level {
            DiagnosticLevel::Error => TxtxError::Execution,
            DiagnosticLevel::Warning => TxtxError::Validation,
            DiagnosticLevel::Note => TxtxError::Validation,
        };

        let mut report = Report::new(base_error).attach_printable(diag.message.clone());

        if let Some(location) = diag.location {
            if let Some(span) = diag.span {
                report = report.attach(ErrorLocation {
                    file: location.to_string(),
                    line: span.line_start,
                    column: span.column_start,
                });
            }
        }

        if diag.documentation.is_some() || diag.example.is_some() {
            report = report.attach(ErrorDocumentation {
                help: diag.documentation.unwrap_or_default(),
                example: diag.example,
                link: None,
            });
        }

        report
    }
}

/// Helper macro for creating errors with context
#[macro_export]
macro_rules! txtx_error {
    ($error:expr, $($arg:tt)*) => {{
        use $crate::types::errors::TxtxError;
        error_stack::Report::new($error)
            .attach_printable(format!($($arg)*))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use error_stack::Report;

    #[test]
    fn test_error_creation() {
        let error: Report<TxtxError> =
            Report::new(TxtxError::TypeMismatch).attach_printable("Expected string but got number");

        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Type mismatch"));
        assert!(error_string.contains("Expected string but got number"));
    }

    #[test]
    fn test_error_with_attachments() {
        let result: Result<(), Report<TxtxError>> =
            Err(Report::new(TxtxError::MissingInput)
                .attach_printable("Input 'address' is required"));

        let error = result
            .with_location("contract.tx", 10, 5)
            .with_documentation("The 'address' field must be a valid Ethereum address")
            .with_example("address = \"0x742d35Cc6634C0532925a3b844Bc9e7595f89590\"")
            .unwrap_err();

        // Verify attachments
        let location = error.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.file, "contract.tx");
        assert_eq!(location.line, 10);
        assert_eq!(location.column, 5);

        let docs = error.downcast_ref::<ErrorDocumentation>().unwrap();
        assert!(docs.help.contains("valid Ethereum address"));
        assert!(docs.example.as_ref().unwrap().contains("0x742d35"));
    }

    #[test]
    fn test_diagnostic_compatibility() {
        let diag = Diagnostic::error_from_string("Test error".to_string());
        let report: Report<TxtxError> = diag.into();

        let error_string = format!("{:?}", report);
        assert!(error_string.contains("Test error"));
    }

    #[test]
    fn test_txtx_error_macro() {
        let error = txtx_error!(TxtxError::Network, "Failed to connect to {}", "mainnet");
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Network operation failed"));
        assert!(error_string.contains("Failed to connect to mainnet"));
    }
}

#[cfg(test)]
#[path = "errors_integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "errors_demo.rs"]
mod demo;
