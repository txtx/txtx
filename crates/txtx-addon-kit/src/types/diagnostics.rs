use std::{any::Any, fmt::Display, ops::Range};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub span: Option<DiagnosticSpan>,
    span_range: Option<Range<usize>>,
    pub location: Option<FileLocation>,
    pub message: String,
    pub level: DiagnosticLevel,
    pub documentation: Option<String>,
    pub example: Option<String>,
    pub parent_diagnostic: Option<Box<Diagnostic>>,
    /// Original error preserved for addons using error-stack
    #[serde(skip)]
    pub source_error: Option<Box<dyn Any + Send + Sync>>,
}

impl Clone for Diagnostic {
    fn clone(&self) -> Self {
        Diagnostic {
            span: self.span.clone(),
            span_range: self.span_range.clone(),
            location: self.location.clone(),
            message: self.message.clone(),
            level: self.level.clone(),
            documentation: self.documentation.clone(),
            example: self.example.clone(),
            parent_diagnostic: self.parent_diagnostic.clone(),
            source_error: None, // Don't clone the source error
        }
    }
}

impl PartialEq for Diagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span
            && self.span_range == other.span_range
            && self.location == other.location
            && self.message == other.message
            && self.level == other.level
            && self.documentation == other.documentation
            && self.example == other.example
            && self.parent_diagnostic == other.parent_diagnostic
        // Ignore source_error in comparison
    }
}

impl Eq for Diagnostic {}

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
            source_error: None,
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
            source_error: None,
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
            source_error: None,
        }
    }

    /// Try to downcast the source error to a specific type
    pub fn downcast_source<T: Any>(&self) -> Option<&T> {
        self.source_error
            .as_ref()
            .and_then(|e| e.downcast_ref::<T>())
    }
    
    /// Check if this diagnostic contains a specific error type
    pub fn has_source_error_type<T: Any>(&self) -> bool {
        self.downcast_source::<T>().is_some()
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
        msg = format!(
            "{}{}{}: {}",
            msg,
            if self.location.is_some() || self.span.is_some() {
                format!("\n\t")
            } else {
                format!("")
            },
            self.level,
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
