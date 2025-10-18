use std::{fmt::Display, ops::Range};

use hcl_edit::{expr::Expression, structure::Block};
use serde::{Deserialize, Serialize};

use crate::helpers::fs::FileLocation;

// Re-export diagnostic types for use and convenience
pub use super::diagnostic_types::{DiagnosticLevel, DiagnosticSpan, RelatedLocation};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub code: Option<String>,
    pub span: Option<DiagnosticSpan>,
    #[serde(skip)]
    span_range: Option<Range<usize>>,
    pub location: Option<FileLocation>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub related_locations: Vec<RelatedLocation>,
    pub documentation: Option<String>,
    pub suggestion: Option<String>,
    pub example: Option<String>,
    pub parent_diagnostic: Option<Box<Diagnostic>>,
}

impl Diagnostic {
    pub fn error_from_expression(
        _block: &Block,
        _expr: Option<&Expression>,
        _message: String,
    ) -> Diagnostic {
        unimplemented!()
    }

    pub fn warning_from_expression(
        _block: &Block,
        _expr: Option<&Expression>,
        _message: String,
    ) -> Diagnostic {
        unimplemented!()
    }

    pub fn note_from_expression(
        _block: &Block,
        _expr: Option<&Expression>,
        _message: String,
    ) -> Diagnostic {
        unimplemented!()
    }

    pub fn error_from_string(message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Error,
            message,
            code: None,
            span: None,
            span_range: None,
            location: None,
            file: None,
            line: None,
            column: None,
            context: None,
            related_locations: Vec::new(),
            documentation: None,
            suggestion: None,
            example: None,
            parent_diagnostic: None,
        }
    }

    pub fn warning_from_string(message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Warning,
            message,
            code: None,
            span: None,
            span_range: None,
            location: None,
            file: None,
            line: None,
            column: None,
            context: None,
            related_locations: Vec::new(),
            documentation: None,
            suggestion: None,
            example: None,
            parent_diagnostic: None,
        }
    }

    pub fn note_from_string(message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Note,
            message,
            code: None,
            span: None,
            span_range: None,
            location: None,
            file: None,
            line: None,
            column: None,
            context: None,
            related_locations: Vec::new(),
            documentation: None,
            suggestion: None,
            example: None,
            parent_diagnostic: None,
        }
    }

    // Builder methods
    pub fn error(message: impl Into<String>) -> Self {
        Self::error_from_string(message.into())
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::warning_from_string(message.into())
    }

    pub fn note(message: impl Into<String>) -> Self {
        Self::note_from_string(message.into())
    }

    pub fn with_code(mut self, code: impl AsRef<str>) -> Self {
        self.code = Some(code.as_ref().to_string());
        self
    }

    pub fn with_file(mut self, file: impl AsRef<str>) -> Self {
        self.file = Some(file.as_ref().to_string());
        self
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    pub fn with_related_location(mut self, related: RelatedLocation) -> Self {
        self.related_locations.push(related);
        self
    }

    pub fn with_span(mut self, span: DiagnosticSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn location(mut self, location: &FileLocation) -> Self {
        self.location = Some(location.clone());
        self
    }

    pub fn is_error(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Error)
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Warning)
    }

    pub fn is_note(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Note)
    }

    pub fn set_span_range(mut self, span: Option<Range<usize>>) -> Self {
        self.span_range = span;
        self
    }

    pub fn span_range(&self) -> Option<Range<usize>> {
        self.span_range.clone()
    }

    pub fn set_diagnostic_span(mut self, span: Option<DiagnosticSpan>) -> Self {
        self.span = span;
        self
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut msg = String::new();

        // Add location if available
        if let Some(location) = &self.location {
            let absolute = location.to_string().replace("./", "");
            msg = format!("{} at {}", self.level, absolute);
        } else if let Some(file) = &self.file {
            msg = format!("{} at {}", self.level, file);
        }

        // Add span if available
        if let Some(span) = &self.span {
            msg = format!("{}:{}:{}", msg, span.line_start, span.column_start);
        } else if let Some(line) = self.line {
            if let Some(column) = self.column {
                msg = format!("{}:{}:{}", msg, line, column);
            } else {
                msg = format!("{}:{}", msg, line);
            }
        }

        // Add error code if available
        let level_with_code = if let Some(code) = &self.code {
            format!("{}[{}]", self.level, code)
        } else {
            format!("{}", self.level)
        };

        msg = format!(
            "{}{}{}: {}",
            msg,
            if !msg.is_empty() { "\n\t" } else { "" },
            level_with_code,
            self.message
        );
        write!(f, "{}", msg)
    }
}

impl From<Diagnostic> for String {
    fn from(diagnostic: Diagnostic) -> Self {
        diagnostic.to_string()
    }
}

impl From<String> for Diagnostic {
    fn from(message: String) -> Self {
        Diagnostic::error_from_string(message)
    }
}

impl From<&str> for Diagnostic {
    fn from(message: &str) -> Self {
        Diagnostic::error_from_string(message.to_string())
    }
}

impl From<std::io::Error> for Diagnostic {
    fn from(err: std::io::Error) -> Self {
        Diagnostic::error_from_string(err.to_string())
    }
}
