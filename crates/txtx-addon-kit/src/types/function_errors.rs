use std::fmt;

use super::types::Type;
use super::diagnostics::Diagnostic;

/// Structured error types for function execution and validation (owned version)
#[derive(Debug, Clone)]
pub enum FunctionError {
    MissingArgument {
        namespace: String,
        function: String,
        position: usize,
        name: String,
    },
    TypeMismatch {
        namespace: String,
        function: String,
        position: usize,
        name: String,
        expected: Vec<Type>,
        found: Type,
    },
    ExecutionError {
        namespace: String,
        function: String,
        message: String,
    },
}

/// Borrowing version for creating errors without allocation
#[derive(Debug)]
pub enum FunctionErrorRef<'a> {
    MissingArgument {
        namespace: &'a str,
        function: &'a str,
        position: usize,
        name: &'a str,
    },
    TypeMismatch {
        namespace: &'a str,
        function: &'a str,
        position: usize,
        name: &'a str,
        expected: &'a [Type],
        found: &'a Type,
    },
    ExecutionError {
        namespace: &'a str,
        function: &'a str,
        message: &'a str,
    },
}

impl fmt::Display for FunctionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionError::MissingArgument { namespace, function, position, name } => {
                write!(
                    f,
                    "function '{}::{}' missing required argument #{} ({})",
                    namespace, function, position, name
                )
            }
            FunctionError::TypeMismatch { namespace, function, position, name, expected, found } => {
                let expected_types = expected
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                write!(
                    f,
                    "function '{}::{}' argument #{} ({}) should be of type ({}), found {}",
                    namespace, function, position, name, expected_types, found
                )
            }
            FunctionError::ExecutionError { namespace, function, message } => {
                write!(f, "function '{}::{}': {}", namespace, function, message)
            }
        }
    }
}

impl<'a> fmt::Display for FunctionErrorRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionErrorRef::MissingArgument { namespace, function, position, name } => {
                write!(
                    f,
                    "function '{}::{}' missing required argument #{} ({})",
                    namespace, function, position, name
                )
            }
            FunctionErrorRef::TypeMismatch { namespace, function, position, name, expected, found } => {
                let expected_types = expected
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                write!(
                    f,
                    "function '{}::{}' argument #{} ({}) should be of type ({}), found {}",
                    namespace, function, position, name, expected_types, found
                )
            }
            FunctionErrorRef::ExecutionError { namespace, function, message } => {
                write!(f, "function '{}::{}': {}", namespace, function, message)
            }
        }
    }
}

impl<'a> From<FunctionErrorRef<'a>> for FunctionError {
    fn from(err: FunctionErrorRef<'a>) -> Self {
        match err {
            FunctionErrorRef::MissingArgument { namespace, function, position, name } => {
                FunctionError::MissingArgument {
                    namespace: namespace.to_string(),
                    function: function.to_string(),
                    position,
                    name: name.to_string(),
                }
            }
            FunctionErrorRef::TypeMismatch { namespace, function, position, name, expected, found } => {
                FunctionError::TypeMismatch {
                    namespace: namespace.to_string(),
                    function: function.to_string(),
                    position,
                    name: name.to_string(),
                    expected: expected.to_vec(),
                    found: found.clone(),
                }
            }
            FunctionErrorRef::ExecutionError { namespace, function, message } => {
                FunctionError::ExecutionError {
                    namespace: namespace.to_string(),
                    function: function.to_string(),
                    message: message.to_string(),
                }
            }
        }
    }
}

impl From<FunctionError> for Diagnostic {
    fn from(err: FunctionError) -> Self {
        Diagnostic::error_from_string(err.to_string())
    }
}

impl<'a> From<FunctionErrorRef<'a>> for Diagnostic {
    fn from(err: FunctionErrorRef<'a>) -> Self {
        Diagnostic::error_from_string(err.to_string())
    }
}