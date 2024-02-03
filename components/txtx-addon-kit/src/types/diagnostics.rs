use crate::helpers::fs::FileLocation;

#[derive(Clone, Debug)]
pub struct DiagnosticSpan {
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
}

#[derive(Clone, Debug)]
pub enum DiagnosticLevel {
    Note,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub span: DiagnosticSpan,
    pub location: FileLocation,
    pub message: String,
    pub level: DiagnosticLevel,
    pub documentation: Option<String>,
    pub example: Option<String>,
    pub parent_diagnostic: Option<Box<Diagnostic>>,
}
