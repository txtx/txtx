//! Common test utilities shared across integration tests

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
