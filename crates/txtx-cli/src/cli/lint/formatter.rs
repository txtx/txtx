//! Output formatting for validation results

use txtx_core::validation::ValidationResult;
use clap;
use colored::Colorize;
use serde_json;
use std::collections::HashMap;
use std::fs;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/// Output format for lint results
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    clap::ValueEnum,  // For CLI argument parsing
    AsRefStr,         // Provides as_ref() -> &str
    Display,          // Provides to_string()
    EnumString,       // Provides from_str()
    IntoStaticStr,    // Provides into() -> &'static str
    EnumIter,         // Provides iter() over all variants
)]
#[strum(serialize_all = "lowercase")]
pub enum Format {
    /// Stylish format (default, human-readable)
    Stylish,
    /// Compact format (one line per issue)
    Compact,
    /// JSON format (machine-readable)
    Json,
    /// Quickfix format (vim-compatible)
    Quickfix,
    /// Documentation format (generates docs from rules)
    Doc,
}

pub trait OutputFormatter {
    fn format(&self, result: &ValidationResult);
}

pub fn get_formatter(format: Format) -> Box<dyn OutputFormatter> {
    match format {
        Format::Stylish => Box::new(StylishFormatter),
        Format::Compact => Box::new(CompactFormatter),
        Format::Json => Box::new(JsonFormatter),
        Format::Quickfix => Box::new(QuickfixFormatter),
        Format::Doc => Box::new(DocumentationFormatter),
    }
}

struct StylishFormatter;

impl OutputFormatter for StylishFormatter {
    fn format(&self, result: &ValidationResult) {
        let total = result.errors.len() + result.warnings.len();

        if total == 0 {
            println!("{}", "✓ No issues found!".green());
            return;
        }

        println!("{}", format!("Found {} issue(s):", total).red().bold());

        for error in &result.errors {
            println!(
                "  {} {} {}",
                "error:".red().bold(),
                error.message,
                error.file.as_deref()
                    .map(|f| format_location(f, error.line, error.column))
                    .unwrap_or_default()
                    .dimmed()
            );

            if let Some(ref context) = error.context {
                println!("    {}", context.dimmed());
            }

            // Display related locations
            for related in &error.related_locations {
                println!(
                    "    {} {}",
                    "→".dimmed(),
                    related.message.dimmed()
                );
                println!(
                    "      {}",
                    format!("at {}", format_location(&related.file, Some(related.line), Some(related.column))).dimmed()
                );
            }
        }

        for warning in &result.warnings {
            println!(
                "  {} {} {}",
                "warning:".yellow().bold(),
                warning.message,
                warning.file.as_deref()
                    .map(|f| format_location(f, warning.line, warning.column))
                    .unwrap_or_default()
                    .dimmed()
            );
        }
    }
}

struct CompactFormatter;

impl OutputFormatter for CompactFormatter {
    fn format(&self, result: &ValidationResult) {
        for error in &result.errors {
            println!(
                "{}:{}:{}: error: {}",
                error.file.as_deref().unwrap_or("<unknown>"),
                error.line.unwrap_or(1),
                error.column.unwrap_or(1),
                error.message
            );
        }

        for warning in &result.warnings {
            println!(
                "{}:{}:{}: warning: {}",
                warning.file.as_deref().unwrap_or("<unknown>"),
                warning.line.unwrap_or(1),
                warning.column.unwrap_or(1),
                warning.message
            );
        }
    }
}

struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, result: &ValidationResult) {
        // Create a custom JSON structure since ValidationResult doesn't implement Serialize
        let output = serde_json::json!({
            "errors": result.errors.iter().map(|e| {
                serde_json::json!({
                    "message": e.message,
                    "file": e.file,
                    "line": e.line,
                    "column": e.column,
                    "context": e.context,
                    "related_locations": e.related_locations.iter().map(|r| {
                        serde_json::json!({
                            "file": r.file,
                            "line": r.line,
                            "column": r.column,
                            "message": r.message,
                        })
                    }).collect::<Vec<_>>(),
                    "documentation": e.documentation,
                })
            }).collect::<Vec<_>>(),
            "warnings": result.warnings.iter().map(|w| {
                serde_json::json!({
                    "message": w.message,
                    "file": w.file,
                    "line": w.line,
                    "column": w.column,
                    "suggestion": w.suggestion,
                })
            }).collect::<Vec<_>>(),
        });

        let json = serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string());
        println!("{}", json);
    }
}

struct QuickfixFormatter;

impl OutputFormatter for QuickfixFormatter {
    fn format(&self, result: &ValidationResult) {
        for error in &result.errors {
            println!(
                "{}:{}:{}: E: {}",
                error.file.as_deref().unwrap_or("<unknown>"),
                error.line.unwrap_or(1),
                error.column.unwrap_or(1),
                error.message
            );
        }

        for warning in &result.warnings {
            println!(
                "{}:{}:{}: W: {}",
                warning.file.as_deref().unwrap_or("<unknown>"),
                warning.line.unwrap_or(1),
                warning.column.unwrap_or(1),
                warning.message
            );
        }
    }
}

fn format_location(file: &str, line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(l), Some(c)) => format!("{}:{}:{}", file, l, c),
        (Some(l), None) => format!("{}:{}", file, l),
        _ => file.to_string(),
    }
}

/// Documentation formatter that renders source code with error squigglies
///
/// Designed for creating shareable examples and documentation. Outputs markdown-compatible
/// code blocks with error annotations using caret indicators (^^^).
///
/// # Example Output
///
/// ```text
/// Error in flows.tx:
///
///    1 | flow "super2" {
///    2 |   api_url = "https://api.com"
///    3 | }
///    4 |
///    5 | action "deploy" {
///    6 |   url = flow.chain_id
///      |              ^^^^^^^^ error: Flow 'super2' missing input 'chain_id'
///    7 | }
/// ```
struct DocumentationFormatter;

impl OutputFormatter for DocumentationFormatter {
    fn format(&self, result: &ValidationResult) {
        // Group errors and warnings by file
        let mut issues_by_file: HashMap<String, Vec<Issue>> = HashMap::new();

        for error in &result.errors {
            let file = error.file.clone().unwrap_or_else(|| "<unknown>".to_string());
            issues_by_file
                .entry(file)
                .or_default()
                .push(Issue {
                    line: error.line,
                    column: error.column,
                    message: error.message.clone(),
                    severity: "error",
                });
        }

        for warning in &result.warnings {
            let file = warning.file.clone().unwrap_or_else(|| "<unknown>".to_string());
            issues_by_file
                .entry(file)
                .or_default()
                .push(Issue {
                    line: warning.line,
                    column: warning.column,
                    message: warning.message.clone(),
                    severity: "warning",
                });
        }

        // Render each file with its issues
        for (file_path, mut issues) in issues_by_file {
            // Sort issues by line number
            issues.sort_by_key(|issue| issue.line.unwrap_or(0));

            println!("\n{}:\n", file_path);

            // Read source file
            let source = match fs::read_to_string(&file_path) {
                Ok(content) => content,
                Err(_) => {
                    // If we can't read the file, just show the errors
                    for issue in issues {
                        println!(
                            "   {} {} {}",
                            format!("{}:", issue.severity).red().bold(),
                            issue.message,
                            format_location(&file_path, issue.line, issue.column).dimmed()
                        );
                    }
                    continue;
                }
            };

            render_source_with_issues(&source, &issues);
        }

        // Summary
        let total = result.errors.len() + result.warnings.len();
        if total == 0 {
            println!("\n{}", "✓ No issues found!".green());
        } else {
            println!("\n{} issue(s) found", total);
        }
    }
}

#[derive(Clone)]
struct Issue {
    line: Option<usize>,
    column: Option<usize>,
    message: String,
    severity: &'static str,
}

/// Render source code with inline error annotations
fn render_source_with_issues(source: &str, issues: &[Issue]) {
    let lines: Vec<&str> = source.lines().collect();
    let max_line_num = lines.len();
    let line_num_width = format!("{}", max_line_num).len();

    // Group issues by line
    let mut issues_by_line: HashMap<usize, Vec<&Issue>> = HashMap::new();
    for issue in issues {
        if let Some(line) = issue.line {
            issues_by_line.entry(line).or_default().push(issue);
        }
    }

    // Determine which lines to show (context around errors)
    let mut lines_to_show = std::collections::HashSet::new();
    for &error_line in issues_by_line.keys() {
        // Show 2 lines before and 2 lines after each error
        for line in error_line.saturating_sub(2)..=(error_line + 2).min(max_line_num) {
            lines_to_show.insert(line);
        }
    }

    let mut prev_line = 0;
    for (idx, line_text) in lines.iter().enumerate() {
        let line_num = idx + 1;

        if !lines_to_show.contains(&line_num) {
            continue;
        }

        // Show ellipsis for skipped lines
        if line_num > prev_line + 1 && prev_line > 0 {
            println!("{:>width$} ⋮", "", width = line_num_width + 3);
        }
        prev_line = line_num;

        // Print line number and source
        println!(
            " {:>width$} │ {}",
            line_num,
            line_text,
            width = line_num_width
        );

        // Print error annotations for this line
        if let Some(line_issues) = issues_by_line.get(&line_num) {
            for issue in line_issues {
                let severity_color = match issue.severity {
                    "error" => "red",
                    "warning" => "yellow",
                    _ => "blue",
                };

                if let Some(col) = issue.column {
                    // Calculate squiggly length based on error message keywords
                    let squiggly_len = estimate_token_length(&issue.message);
                    let padding = " ".repeat(col.saturating_sub(1));
                    let squigglies = "^".repeat(squiggly_len);

                    let annotation = format!(
                        " {:>width$} │ {}{} {}: {}",
                        "",
                        padding,
                        squigglies,
                        issue.severity,
                        issue.message,
                        width = line_num_width
                    );

                    println!("{}", match severity_color {
                        "red" => annotation.red(),
                        "yellow" => annotation.yellow(),
                        _ => annotation.blue(),
                    });
                } else {
                    // No column info, just show message
                    println!(
                        " {:>width$} │ {}: {}",
                        "",
                        issue.severity,
                        issue.message,
                        width = line_num_width
                    );
                }
            }
        }
    }
}

/// Estimate the length of the token causing the error based on error message
fn estimate_token_length(message: &str) -> usize {
    // Look for quoted identifiers in the message
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            return end;
        }
    }

    // Default squiggly length
    8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_format_display() {
        assert_eq!(Format::Stylish.to_string(), "stylish");
        assert_eq!(Format::Compact.to_string(), "compact");
        assert_eq!(Format::Json.to_string(), "json");
        assert_eq!(Format::Quickfix.to_string(), "quickfix");
        assert_eq!(Format::Doc.to_string(), "doc");
    }

    #[test]
    fn test_format_from_str() {
        // Test successful parsing
        assert_eq!(Format::from_str("stylish").unwrap(), Format::Stylish);
        assert_eq!(Format::from_str("compact").unwrap(), Format::Compact);
        assert_eq!(Format::from_str("json").unwrap(), Format::Json);
        assert_eq!(Format::from_str("quickfix").unwrap(), Format::Quickfix);
        assert_eq!(Format::from_str("doc").unwrap(), Format::Doc);

        // Test invalid input
        assert!(Format::from_str("invalid").is_err());
    }

    #[test]
    fn test_format_iteration() {
        use strum::IntoEnumIterator;

        let all_formats: Vec<Format> = Format::iter().collect();
        assert_eq!(all_formats.len(), 5);
        assert!(all_formats.contains(&Format::Stylish));
        assert!(all_formats.contains(&Format::Compact));
        assert!(all_formats.contains(&Format::Json));
        assert!(all_formats.contains(&Format::Quickfix));
        assert!(all_formats.contains(&Format::Doc));
    }

    #[test]
    fn test_format_as_ref() {
        // Test AsRefStr trait
        assert_eq!(Format::Stylish.as_ref(), "stylish");
        assert_eq!(Format::Json.as_ref(), "json");
    }

    #[test]
    fn test_format_location_with_line_and_column() {
        // Arrange
        let file = "test.tx";
        let line = Some(10);
        let column = Some(5);

        // Act
        let result = format_location(file, line, column);

        // Assert
        assert_eq!(result, "test.tx:10:5");
    }

    #[test]
    fn test_format_location_with_line_only() {
        // Arrange
        let file = "test.tx";
        let line = Some(10);
        let column = None;

        // Act
        let result = format_location(file, line, column);

        // Assert
        assert_eq!(result, "test.tx:10");
    }

    #[test]
    fn test_format_location_with_no_position() {
        // Arrange
        let file = "test.tx";
        let line = None;
        let column = None;

        // Act
        let result = format_location(file, line, column);

        // Assert
        assert_eq!(result, "test.tx");
    }

    #[test]
    fn test_estimate_token_length_with_quoted_identifier() {
        // Arrange
        let message = "Undefined variable 'test_var' in expression";

        // Act
        let length = estimate_token_length(message);

        // Assert
        assert_eq!(length, 8); // Length of "test_var"
    }

    #[test]
    fn test_estimate_token_length_without_quotes() {
        // Arrange
        let message = "Syntax error in expression";

        // Act
        let length = estimate_token_length(message);

        // Assert
        assert_eq!(length, 8); // Default length
    }

    // Property-based tests
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use txtx_addon_kit::types::diagnostics::Diagnostic;
        use std::io::Write;

        // Generate arbitrary diagnostics
        prop_compose! {
            fn arb_diagnostic()(
                message in "[a-zA-Z0-9 !@#$%^&*()_+=\\[\\]{}|;:',.<>?/`~\\-]{1,200}",  // ASCII printable chars, 1-200 chars
                line in 1usize..1000,
                column in 0usize..200,
                file in prop::option::of("[a-zA-Z0-9_/.\\-]{1,50}"),  // ASCII filename chars
                code in prop::option::of("[A-Z0-9_\\-]{1,20}"),  // ASCII code chars (uppercase, numbers, underscore, hyphen)
                context in prop::option::of("[a-zA-Z0-9 !@#$%^&*()_+=\\[\\]{}|;:',.<>?/`~\\-]{1,100}"),  // ASCII context
            ) -> Diagnostic {
                let mut diag = Diagnostic::error(message);
                diag.line = Some(line);
                diag.column = Some(column);
                if let Some(f) = file {
                    diag.file = Some(f);
                }
                if let Some(c) = code {
                    diag.code = Some(c);
                }
                if let Some(ctx) = context {
                    diag.context = Some(ctx);
                }
                diag
            }
        }

        // Generate arbitrary validation results
        prop_compose! {
            fn arb_validation_result()(
                errors in prop::collection::vec(arb_diagnostic(), 0..10),
                warnings in prop::collection::vec(arb_diagnostic(), 0..10),
            ) -> ValidationResult {
                ValidationResult {
                    errors,
                    warnings,
                    suggestions: vec![],
                }
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Property: Formatters should never panic on any ValidationResult
            #[test]
            fn formatters_never_panic(result in arb_validation_result()) {
                // Test each formatter doesn't panic
                assert!(std::panic::catch_unwind(|| {
                    StylishFormatter.format(&result);
                }).is_ok(), "StylishFormatter should not panic");

                assert!(std::panic::catch_unwind(|| {
                    CompactFormatter.format(&result);
                }).is_ok(), "CompactFormatter should not panic");

                assert!(std::panic::catch_unwind(|| {
                    JsonFormatter.format(&result);
                }).is_ok(), "JsonFormatter should not panic");

                assert!(std::panic::catch_unwind(|| {
                    QuickfixFormatter.format(&result);
                }).is_ok(), "QuickfixFormatter should not panic");

                assert!(std::panic::catch_unwind(|| {
                    DocumentationFormatter.format(&result);
                }).is_ok(), "DocumentationFormatter should not panic");
            }

            /// Property: format_location should handle all combinations of line/column
            #[test]
            fn format_location_handles_all_combinations(
                file in "[a-zA-Z0-9_/.\\-]{1,50}",
                line in prop::option::of(1usize..10000),
                column in prop::option::of(0usize..500)
            ) {
                // Act
                let result = format_location(&file, line, column);

                // Assert - verify format based on what was provided
                match (line, column) {
                    (Some(l), Some(c)) => {
                        prop_assert_eq!(result, format!("{}:{}:{}", file, l, c));
                    }
                    (Some(l), None) => {
                        prop_assert_eq!(result, format!("{}:{}", file, l));
                    }
                    _ => {
                        prop_assert_eq!(result, file);
                    }
                }
            }

            /// Property: estimate_token_length should find quoted strings or return default
            #[test]
            fn estimate_token_length_finds_quoted_strings(
                prefix in "[a-zA-Z ]{0,50}",
                token in "[a-zA-Z0-9_]{1,30}",
                suffix in "[a-zA-Z ]{0,50}"
            ) {
                // Arrange - create message with quoted token
                let message = format!("{}'{}'{}", prefix, token, suffix);

                // Act
                let length = estimate_token_length(&message);

                // Assert
                prop_assert_eq!(length, token.len(),
                    "Should return length of quoted token '{}'", token);
            }

            /// Property: estimate_token_length returns default for unquoted messages
            #[test]
            fn estimate_token_length_default_for_unquoted(
                message in "[a-zA-Z0-9 ]{1,100}"
            ) {
                // Skip if message accidentally contains quotes
                prop_assume!(!message.contains('\''));

                // Act
                let length = estimate_token_length(&message);

                // Assert
                prop_assert_eq!(length, 8, "Should return default length for unquoted message");
            }
        }
    }
}
