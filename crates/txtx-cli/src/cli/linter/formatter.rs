//! Output formatting for validation results

use txtx_core::validation::ValidationResult;
use colored::Colorize;
use serde_json;
use std::collections::HashMap;
use std::fs;

#[derive(Clone, Copy, Debug)]
pub enum Format {
    Stylish,
    Compact,
    Json,
    Quickfix,
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
                format_location(&error.file, error.line, error.column).dimmed()
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
                format_location(
                    &warning.file,
                    warning.line,
                    warning.column
                ).dimmed()
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
                error.file,
                error.line.unwrap_or(1),
                error.column.unwrap_or(1),
                error.message
            );
        }

        for warning in &result.warnings {
            let file = &warning.file;
            println!(
                "{}:{}:{}: warning: {}",
                file,
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
                    "documentation_link": e.documentation_link,
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
                error.file,
                error.line.unwrap_or(1),
                error.column.unwrap_or(1),
                error.message
            );
        }

        for warning in &result.warnings {
            let file = &warning.file;
            println!(
                "{}:{}:{}: W: {}",
                file,
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
            issues_by_file
                .entry(error.file.clone())
                .or_default()
                .push(Issue {
                    line: error.line,
                    column: error.column,
                    message: error.message.clone(),
                    severity: "error",
                });
        }

        for warning in &result.warnings {
            issues_by_file
                .entry(warning.file.clone())
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