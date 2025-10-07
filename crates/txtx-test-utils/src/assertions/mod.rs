//! Common assertion macros for txtx tests

/// Assert that a result contains a specific error pattern
#[macro_export]
macro_rules! assert_error {
    ($result:expr, $pattern:expr) => {
        match &$result {
            Ok(_) => panic!("Expected error containing '{}', but got success", $pattern),
            Err(e) => {
                let error_str = e.to_string();
                assert!(
                    error_str.contains($pattern),
                    "Expected error containing '{}', but got: {}",
                    $pattern,
                    error_str
                );
            }
        }
    };
}

/// Assert that a validation result contains a specific error
#[macro_export]
macro_rules! assert_validation_error {
    ($result:expr, $pattern:expr) => {
        assert!(!$result.success, "Expected validation error, but validation succeeded");
        let errors_str =
            $result.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
        assert!(
            errors_str.contains($pattern),
            "Expected error containing '{}', but got:\n{}",
            $pattern,
            errors_str
        );
    };
}

/// Assert that a parse result failed
#[macro_export]
macro_rules! assert_parse_error {
    ($result:expr) => {
        assert!(!$result.success, "Expected parse error, but parsing succeeded");
    };
    ($result:expr, $pattern:expr) => {
        assert!(!$result.success, "Expected parse error, but parsing succeeded");
        let errors_str =
            $result.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
        assert!(
            errors_str.contains($pattern),
            "Expected error containing '{}', but got:\n{}",
            $pattern,
            errors_str
        );
    };
}

/// Assert that validation warning contains pattern
#[macro_export]
macro_rules! assert_validation_warning {
    ($result:expr, $pattern:expr) => {
        let pattern = $pattern;
        let found = $result.warnings.iter().any(|w| w.message.contains(pattern));
        if !found {
            let warnings_str = $result
                .warnings
                .iter()
                .map(|w| format!("  - {}", w.message))
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "Expected warning containing '{}', but got:\n{}",
                pattern,
                if warnings_str.is_empty() { "  (no warnings)".to_string() } else { warnings_str }
            );
        }
    };
}

/// Assert that execution succeeded
#[macro_export]
macro_rules! assert_success {
    ($result:expr) => {
        if !$result.success {
            let errors_str =
                $result.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            panic!("Expected success, but got errors:\n{}", errors_str);
        }
    };
}

/// Assert that an output value matches
#[macro_export]
macro_rules! assert_output {
    ($result:expr, $key:expr, $value:expr) => {
        assert_success!($result);
        assert_eq!(
            $result.outputs.get($key),
            Some(&$value.to_string()),
            "Output '{}' mismatch",
            $key
        );
    };
}

#[cfg(test)]
mod tests {
    use crate::builders::{ExecutionResult, ValidationResult};
    use txtx_addon_kit::types::diagnostics::Diagnostic;

    #[test]
    fn test_assert_validation_error() {
        let result = ValidationResult {
            success: false,
            errors: vec![Diagnostic::error_from_string("undefined variable: foo".to_string())],
            warnings: vec![],
        };

        assert_validation_error!(result, "undefined variable");
    }

    #[test]
    fn test_assert_success() {
        let result = ExecutionResult {
            success: true,
            outputs: [("test".to_string(), "value".to_string())].into(),
            errors: vec![],
        };

        assert_success!(result);
        assert_output!(result, "test", "value");
    }
}
