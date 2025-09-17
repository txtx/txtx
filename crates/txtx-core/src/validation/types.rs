use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LocatedInputRef {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub suggestions: Vec<ValidationSuggestion>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationError {
    pub message: String,
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub documentation_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationWarning {
    pub message: String,
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationSuggestion {
    pub message: String,
    pub example: Option<String>,
}
