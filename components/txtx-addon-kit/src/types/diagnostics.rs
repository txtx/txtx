use std::fmt::Display;

use hcl_edit::{expr::Expression, structure::Block};

use crate::helpers::fs::FileLocation;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Option<DiagnosticSpan>,
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
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.level, self.message)
    }
}
