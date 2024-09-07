use std::{fmt::Display, ops::Range};

use hcl_edit::{expr::Expression, structure::Block};

use crate::helpers::fs::FileLocation;

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
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Option<DiagnosticSpan>,
    span_range: Option<Range<usize>>,
    pub location: Option<FileLocation>,
    pub message: String,
    pub level: DiagnosticLevel,
    pub documentation: Option<String>,
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
            span: None,
            span_range: None,
            location: None,
            message,
            level: DiagnosticLevel::Error,
            documentation: None,
            example: None,
            parent_diagnostic: None,
        }
    }
    pub fn warning_from_string(message: String) -> Diagnostic {
        Diagnostic {
            span: None,
            span_range: None,
            location: None,
            message,
            level: DiagnosticLevel::Warning,
            documentation: None,
            example: None,
            parent_diagnostic: None,
        }
    }
    pub fn note_from_string(message: String) -> Diagnostic {
        Diagnostic {
            span: None,
            span_range: None,
            location: None,
            message,
            level: DiagnosticLevel::Note,
            documentation: None,
            example: None,
            parent_diagnostic: None,
        }
    }

    pub fn location(mut self, location: &FileLocation) -> Self {
        self.location = Some(location.clone());
        self
    }

    pub fn is_error(&self) -> bool {
        if let DiagnosticLevel::Error = self.level {
            true
        } else {
            false
        }
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
        if let Some(location) = &self.location {
            let absolute = location.to_string().replace("./", "");
            msg = format!("{} at {}", self.level, absolute);
        }
        if let Some(span) = &self.span {
            msg = format!("{}:{}:{}", msg, span.line_start, span.column_start);
        }
        msg = format!("{}\n\t{}: {}", msg, self.level, self.message);
        write!(f, "{}", msg)
    }
}
