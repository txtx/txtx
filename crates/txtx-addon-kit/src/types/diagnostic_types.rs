use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Severity level for diagnostics
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Note,
    Warning,
    Error,
}

impl Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "error"),
            DiagnosticLevel::Warning => write!(f, "warning"),
            DiagnosticLevel::Note => write!(f, "note"),
        }
    }
}

/// Span information with line/column ranges
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
}

impl DiagnosticSpan {
    pub fn new() -> Self {
        DiagnosticSpan { line_start: 0, line_end: 0, column_start: 0, column_end: 0 }
    }

    pub fn from_line_column(line: usize, column: usize) -> Self {
        DiagnosticSpan {
            line_start: line as u32,
            line_end: line as u32,
            column_start: column as u32,
            column_end: column as u32,
        }
    }
}

/// A related location that provides additional context
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RelatedLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl RelatedLocation {
    pub fn new(file: String, line: usize, column: usize, message: String) -> Self {
        Self { file, line, column, message }
    }
}
