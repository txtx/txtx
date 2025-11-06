//! Shared test utilities for linter tests
//!
//! This module provides assertion macros for validating linter results.
//! These macros are used by both unit tests and integration tests.

#[cfg(test)]
pub mod manifest_builders {
    use std::collections::HashMap;
    use txtx_core::manifest::WorkspaceManifest;

    /// Create a basic test manifest with default values
    pub fn create_test_manifest() -> WorkspaceManifest {
        WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        }
    }

    /// Create a test manifest with global environment inputs
    pub fn create_manifest_with_global(inputs: &[(&str, &str)]) -> WorkspaceManifest {
        let mut manifest = create_test_manifest();
        let env: HashMap<String, String> = inputs.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        manifest.environments.insert("global".to_string(), env.into_iter().collect());
        manifest
    }

    /// Create a test manifest with a named environment
    pub fn create_manifest_with_env(env_name: &str, inputs: &[(&str, &str)]) -> WorkspaceManifest {
        let mut manifest = create_test_manifest();
        let env: HashMap<String, String> = inputs.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        manifest.environments.insert(env_name.to_string(), env.into_iter().collect());
        manifest
    }

    /// Add global environment to an existing manifest
    pub fn with_global_env(mut manifest: WorkspaceManifest, inputs: &[(&str, &str)]) -> WorkspaceManifest {
        let env: HashMap<String, String> = inputs.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        manifest.environments.insert("global".to_string(), env.into_iter().collect());
        manifest
    }

    /// Add a named environment to an existing manifest
    pub fn with_env(mut manifest: WorkspaceManifest, env_name: &str, inputs: &[(&str, &str)]) -> WorkspaceManifest {
        let env: HashMap<String, String> = inputs.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        manifest.environments.insert(env_name.to_string(), env.into_iter().collect());
        manifest
    }
}

/// Assert that a violation occurs at a specific location
#[macro_export]
macro_rules! assert_violation_at {
    ($result:expr, $code:expr, $line:expr) => {
        let violations: Vec<_> = $result.errors.iter()
            .chain($result.warnings.iter())
            .filter(|v| v.code.as_deref() == Some($code))
            .collect();

        assert!(
            violations.iter().any(|v| v.line == Some($line)),
            "Expected violation with code '{}' at line {}, but found violations at lines: {:?}",
            $code,
            $line,
            violations.iter().map(|v| v.line).collect::<Vec<_>>()
        );
    };
    ($result:expr, $code:expr, $line:expr, $col:expr) => {
        let violations: Vec<_> = $result.errors.iter()
            .chain($result.warnings.iter())
            .filter(|v| v.code.as_deref() == Some($code))
            .collect();

        assert!(
            violations.iter().any(|v| v.line == Some($line) && v.column == Some($col)),
            "Expected violation with code '{}' at line {}, column {}, but found violations at: {:?}",
            $code,
            $line,
            $col,
            violations.iter().map(|v| (v.line, v.column)).collect::<Vec<_>>()
        );
    };
}

/// Assert that there are no violations (errors or warnings) with the given code
#[macro_export]
macro_rules! assert_no_violations {
    ($result:expr, $code:expr) => {
        let violations: Vec<_> = $result.errors.iter()
            .chain($result.warnings.iter())
            .filter(|v| v.code.as_deref() == Some($code))
            .collect();

        assert!(
            violations.is_empty(),
            "Expected no violations with code '{}', but found: {:?}",
            $code,
            violations.iter().map(|v| &v.message).collect::<Vec<_>>()
        );
    };
}

/// Assert that there are exactly N violations with the given code
#[macro_export]
macro_rules! assert_violation_count {
    ($result:expr, $code:expr, $count:expr) => {
        let violations: Vec<_> = $result.errors.iter()
            .chain($result.warnings.iter())
            .filter(|v| v.code.as_deref() == Some($code))
            .collect();

        assert_eq!(
            violations.len(),
            $count,
            "Expected {} violations with code '{}', but found {}: {:?}",
            $count,
            $code,
            violations.len(),
            violations.iter().map(|v| &v.message).collect::<Vec<_>>()
        );
    };
}

/// Assert that a violation contains specific text in its message
#[macro_export]
macro_rules! assert_violation_message_contains {
    ($result:expr, $code:expr, $text:expr) => {
        let violations: Vec<_> = $result.errors.iter()
            .chain($result.warnings.iter())
            .filter(|v| v.code.as_deref() == Some($code))
            .collect();

        assert!(
            violations.iter().any(|v| v.message.contains($text)),
            "Expected violation with code '{}' to contain '{}', but messages were: {:?}",
            $code,
            $text,
            violations.iter().map(|v| &v.message).collect::<Vec<_>>()
        );
    };
}
