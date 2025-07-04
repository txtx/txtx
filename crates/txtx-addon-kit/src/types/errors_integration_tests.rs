#[cfg(test)]
mod integration_tests {
    use crate::types::diagnostics::Diagnostic;
    use crate::types::errors::*;
    use error_stack::{Report, ResultExt};

    /// Simulate a complete error flow from parsing to execution
    #[test]
    fn test_error_flow_with_context() {
        // Simulate parsing phase
        fn parse_value(input: &str) -> Result<i32, Report<TxtxError>> {
            input
                .parse::<i32>()
                .map_err(|_| {
                    Report::new(TxtxError::Parsing)
                        .attach_printable(format!("Failed to parse '{}' as integer", input))
                })
                .with_documentation("Value must be a valid integer")
                .with_example("42")
        }

        // Simulate validation phase
        fn validate_value(value: i32) -> Result<i32, Report<TxtxError>> {
            if value < 0 {
                return Err(Report::new(TxtxError::Validation)
                    .attach_printable("Value must be non-negative")
                    .attach(TypeMismatchInfo {
                        field: "value".to_string(),
                        expected: "non-negative integer".to_string(),
                        actual: format!("negative integer ({})", value),
                    }));
            }
            Ok(value)
        }

        // Simulate execution phase
        fn execute_with_value(value: i32) -> Result<String, Report<TxtxError>> {
            validate_value(value)
                .change_context(TxtxError::Execution)
                .attach_printable("Validation failed during execution")
                .with_action_context("process_value", "test", "test_construct_123")?;

            Ok(format!("Processed value: {}", value))
        }

        // Test success case
        let result = parse_value("42").and_then(execute_with_value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed value: 42");

        // Test parsing error
        let error = parse_value("not_a_number").unwrap_err();
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Failed to parse"));
        assert!(error_string.contains("not_a_number"));

        // Check documentation attachment
        let docs = error.downcast_ref::<ErrorDocumentation>().unwrap();
        assert_eq!(docs.help, "Value must be a valid integer");
        assert_eq!(docs.example.as_ref().unwrap(), "42");

        // Test validation error
        let error = parse_value("-5").and_then(execute_with_value).unwrap_err();

        let type_info = error.downcast_ref::<TypeMismatchInfo>().unwrap();
        assert_eq!(type_info.field, "value");
        assert!(type_info.actual.contains("-5"));

        let action_ctx = error.downcast_ref::<ActionContext>().unwrap();
        assert_eq!(action_ctx.action_name, "process_value");
        assert_eq!(action_ctx.namespace, "test");
    }

    /// Test error attachment accumulation
    #[test]
    fn test_error_attachment_accumulation() {
        let base_error = Report::new(TxtxError::Network).attach_printable("Connection failed");

        let enhanced_error = Err::<(), Report<TxtxError>>(base_error)
            .with_location("network.tx", 25, 10)
            .with_documentation("Check your network connection")
            .with_example("network_url = \"https://mainnet.infura.io/v3/YOUR_KEY\"")
            .with_link("https://docs.txtx.io/network-setup")
            .unwrap_err();

        // Verify all attachments are present
        assert!(enhanced_error.downcast_ref::<ErrorLocation>().is_some());
        assert!(enhanced_error.downcast_ref::<ErrorDocumentation>().is_some());

        let location = enhanced_error.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.file, "network.tx");
        assert_eq!(location.line, 25);

        let docs = enhanced_error.downcast_ref::<ErrorDocumentation>().unwrap();
        assert!(docs.help.contains("network connection"));
        assert!(docs.link.as_ref().unwrap().contains("network-setup"));
    }

    /// Test diagnostic to error-stack conversion
    #[test]
    fn test_diagnostic_migration() {
        use crate::helpers::fs::FileLocation;
        use crate::types::diagnostics::DiagnosticSpan;

        // Create a rich diagnostic
        let mut diagnostic = Diagnostic::error_from_string("Original error message".to_string());
        diagnostic.location = Some(FileLocation::from_path_string("test.tx").unwrap());
        diagnostic.span =
            Some(DiagnosticSpan { line_start: 10, line_end: 10, column_start: 5, column_end: 15 });
        diagnostic.documentation = Some("This is helpful documentation".to_string());
        diagnostic.example = Some("example = \"value\"".to_string());

        // Convert to error-stack
        let report: Report<TxtxError> = diagnostic.into();

        // Verify conversion preserved information
        let error_string = format!("{:?}", report);
        assert!(error_string.contains("Original error message"));

        let location = report.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.line, 10);
        assert_eq!(location.column, 5);

        let docs = report.downcast_ref::<ErrorDocumentation>().unwrap();
        assert_eq!(docs.help, "This is helpful documentation");
        assert_eq!(docs.example.as_ref().unwrap(), "example = \"value\"");
    }

    /// Test error display formatting
    #[test]
    fn test_error_display() {
        let error = Report::new(TxtxError::TypeMismatch)
            .attach_printable("Expected string but got number")
            .attach(TypeMismatchInfo {
                field: "address".to_string(),
                expected: "string".to_string(),
                actual: "number".to_string(),
            })
            .attach(ErrorLocation { file: "contract.tx".to_string(), line: 15, column: 8 });

        // Test Display for individual components
        let type_info = error.downcast_ref::<TypeMismatchInfo>().unwrap();
        assert_eq!(type_info.to_string(), "field 'address' expected string but got number");

        let location = error.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.to_string(), "at contract.tx:15:8");
    }

    /// Test complex error scenario with multiple phases
    #[test]
    fn test_multi_phase_error_handling() {
        #[derive(Debug)]
        enum ProcessingPhase {
            Input,
            Transform,
            Output,
        }

        fn process_data(input: &str) -> Result<String, Report<TxtxError>> {
            // Input phase
            let parsed = input
                .parse::<i32>()
                .map_err(|_| Report::new(TxtxError::Parsing))
                .attach_printable(format!("Phase: {:?}", ProcessingPhase::Input))
                .with_documentation("Input must be a valid integer")?;

            // Transform phase
            let transformed = if parsed > 100 {
                Err(Report::new(TxtxError::Validation)
                    .attach_printable(format!("Phase: {:?}", ProcessingPhase::Transform))
                    .attach_printable("Value too large for transformation"))
            } else {
                Ok(parsed * 2)
            }?;

            // Output phase
            if transformed > 150 {
                return Err(Report::new(TxtxError::Execution)
                    .attach_printable(format!("Phase: {:?}", ProcessingPhase::Output))
                    .attach_printable("Output exceeds maximum allowed value"));
            }

            Ok(format!("Result: {}", transformed))
        }

        // Test various scenarios
        assert!(process_data("50").is_ok());

        let parse_error = process_data("not_a_number").unwrap_err();
        assert!(format!("{:?}", parse_error).contains("Input"));

        let transform_error = process_data("200").unwrap_err();
        assert!(format!("{:?}", transform_error).contains("Transform"));
        assert!(format!("{:?}", transform_error).contains("too large"));
    }
}
