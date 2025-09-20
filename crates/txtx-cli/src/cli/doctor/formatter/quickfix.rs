use txtx_core::validation::ValidationResult;

/// Display results in quickfix format (single line per issue)
/// This format is recognized by most IDEs and editors for quick navigation
pub fn display(result: &ValidationResult) {
    // Errors in quickfix format
    for error in &result.errors {
        let location = format_location(&error.file, error.line, error.column);
        let mut message = format!("error: {}", error.message);

        if let Some(link) = &error.documentation_link {
            message.push_str(&format!(" (see: {})", link));
        }

        println!("{}{}", location, message);
    }

    // Warnings in quickfix format
    for warning in &result.warnings {
        let location = format_location(&warning.file, warning.line, warning.column);
        let mut message = format!("warning: {}", warning.message);

        if let Some(suggestion) = &warning.suggestion {
            message.push_str(&format!(" (hint: {})", suggestion));
        }

        println!("{}{}", location, message);
    }
}

/// Format location in quickfix format: file:line:column:
fn format_location(file: &str, line: Option<usize>, column: Option<usize>) -> String {
    if let (Some(line), Some(column)) = (line, column) {
        format!("{}:{}:{}: ", file, line + 1, column + 1)
    } else {
        // When we don't have specific location, default to line 1 for navigation
        format!("{}:1: ", file)
    }
}
