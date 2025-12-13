use serde::{Deserialize, Serialize};
use super::file_boundary::FileBoundaryMap;

// Use common types from txtx-addon-kit
use txtx_addon_kit::types::diagnostics::Diagnostic;

// Re-export for convenience
pub use txtx_addon_kit::types::diagnostic_types::RelatedLocation;

#[derive(Debug, Clone)]
pub struct LocatedInputRef {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

/// A located signer reference with its type and attributes
#[derive(Debug, Clone)]
pub struct LocatedSignerRef {
    pub name: String,
    pub signer_type: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub line: usize,
    pub column: usize,
}

/// Result of HCL validation containing located references
#[derive(Debug, Clone, Default)]
pub struct HclValidationRefs {
    pub inputs: Vec<LocatedInputRef>,
    pub signers: Vec<LocatedSignerRef>,
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
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

    /// Map error and warning locations from combined file lines to original source files
    ///
    /// This is used when validating multi-file runbooks that have been concatenated.
    /// The boundary map tracks which lines belong to which original files.
    pub fn map_errors_to_source_files(&mut self, boundary_map: &FileBoundaryMap) {
        // Map errors
        for error in &mut self.errors {
            if let Some(line) = error.line {
                let (file, mapped_line) = boundary_map.map_line(line);
                error.file = Some(file);
                error.line = Some(mapped_line);
            }

            // Also map related_locations
            for related in &mut error.related_locations {
                let (file, mapped_line) = boundary_map.map_line(related.line);
                related.file = file;
                related.line = mapped_line;
            }
        }

        // Map warnings
        for warning in &mut self.warnings {
            if let Some(line) = warning.line {
                let (file, mapped_line) = boundary_map.map_line(line);
                warning.file = Some(file);
                warning.line = Some(mapped_line);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationSuggestion {
    pub message: String,
    pub example: Option<String>,
}
