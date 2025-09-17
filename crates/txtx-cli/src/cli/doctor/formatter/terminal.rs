use ansi_term::Colour::{Blue, Red, Yellow};
use txtx_core::validation::ValidationResult;

/// Display results in pretty format (human-readable with colors)
pub fn display(result: &ValidationResult) {
    let total_issues = result.errors.len() + result.warnings.len();

    if total_issues == 0 {
        println!("{} No issues found!", Blue.paint("✓"));
        return;
    }

    println!("{}", Red.bold().paint(format!("Found {} issue(s):", total_issues)));
    println!();

    display_errors(result);
    display_warnings(result);
    display_suggestions(result);
}

/// Display errors with formatting and colors
fn display_errors(result: &ValidationResult) {
    for (i, error) in result.errors.iter().enumerate() {
        let location = format_location(&error.file, error.line, error.column);

        println!(
            "{}{} {}",
            location,
            Red.bold().paint(format!("error[{}]:", i + 1)),
            Red.paint(&error.message)
        );

        if let Some(context) = &error.context {
            println!("   {}", context);
        }

        if let Some(link) = &error.documentation_link {
            println!("   {} {}", Blue.paint("Documentation:"), link);
        }

        println!();
    }
}

/// Display warnings with formatting and colors
fn display_warnings(result: &ValidationResult) {
    for warning in &result.warnings {
        let location = format_location(&warning.file, warning.line, warning.column);

        println!("{}{} {}", location, Yellow.paint("warning:"), warning.message);

        if let Some(suggestion) = &warning.suggestion {
            println!("   {} {}", Blue.paint("Suggestion:"), suggestion);
        }
        println!();
    }
}

/// Display suggestions section
fn display_suggestions(result: &ValidationResult) {
    if !result.suggestions.is_empty() {
        println!("{}", Blue.bold().paint("Suggestions:"));
        for suggestion in &result.suggestions {
            println!("  • {}", suggestion.message);
            if let Some(example) = &suggestion.example {
                println!("    {}", example);
            }
        }
    }
}

/// Format location for clickable IDE integration
fn format_location(file: &str, line: Option<usize>, column: Option<usize>) -> String {
    if let (Some(line), Some(column)) = (line, column) {
        format!("{}:{}:{}: ", file, line + 1, column + 1)
    } else {
        format!("{}: ", file)
    }
}
