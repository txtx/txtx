use txtx_core::validation::ValidationResult;

pub mod json;
pub mod quickfix;
pub mod terminal;

/// Display validation results based on the specified format
pub fn display_results(result: &ValidationResult, format: &crate::cli::DoctorOutputFormat) {
    match format {
        crate::cli::DoctorOutputFormat::Quickfix => quickfix::display(result),
        crate::cli::DoctorOutputFormat::Json => json::display(result),
        crate::cli::DoctorOutputFormat::Pretty => terminal::display(result),
        crate::cli::DoctorOutputFormat::Auto => terminal::display(result), // Should not reach here after auto-detection
    }
}
