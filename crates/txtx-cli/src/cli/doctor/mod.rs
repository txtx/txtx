mod analyzer;
mod config;
mod formatter;
mod workspace;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_core::manifest::{file::read_runbook_from_location, WorkspaceManifest};

use self::{
    config::DoctorConfig,
    formatter::display_results,
    workspace::{RunbookLocation, WorkspaceAnalyzer},
};

/// Main entry point for the doctor command
pub fn run_doctor(
    manifest_path: Option<String>,
    runbook_name: Option<String>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
    format: crate::cli::DoctorOutputFormat,
) -> Result<(), String> {
    // Create and resolve configuration
    let config = DoctorConfig::new(manifest_path, runbook_name, environment, cli_inputs, format)
        .resolve_format();

    if config.should_print_diagnostics() {
        eprintln!("Doctor command running with runbook: {:?}", config.runbook_name);
    }

    // Run the doctor analysis
    match config.runbook_name {
        Some(ref name) => run_specific_runbook(&config, name),
        None => run_all_runbooks(&config),
    }
}

/// Run doctor on a specific runbook
fn run_specific_runbook(config: &DoctorConfig, runbook_name: &str) -> Result<(), String> {
    let workspace = WorkspaceAnalyzer::new(config.manifest_path.clone());
    let analyzer = analyzer::RunbookAnalyzer::new();

    // First try as a direct file path
    let path = PathBuf::from(runbook_name);
    if path.exists() && path.extension().map_or(false, |ext| ext == "tx") {
        // When analyzing a direct file without manifest context
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
        let result = analyzer.analyze_runbook_with_context(&path, &content, None, None, &[]);
        display_results(&result, &config.format);
        return if result.errors.is_empty() {
            Ok(())
        } else {
            Err("Doctor found errors in your runbook".to_string())
        };
    }

    // Try to load from manifest
    match crate::cli::runbooks::load_workspace_manifest_from_manifest_path(
        config.manifest_path.to_str().unwrap(),
    ) {
        Ok(manifest) => {
            if let Some(location) = workspace.find_runbook_in_manifest(&manifest, runbook_name) {
                analyze_runbook_with_manifest(&analyzer, &location, &manifest, config)
            } else {
                Err(format!("Runbook '{}' not found in manifest", runbook_name))
            }
        }
        Err(_) => Err(format!("File '{}' not found or is not a .tx file", runbook_name)),
    }
}

/// Run doctor on all runbooks
fn run_all_runbooks(config: &DoctorConfig) -> Result<(), String> {
    let workspace = WorkspaceAnalyzer::new(config.manifest_path.clone());
    let analyzer = analyzer::RunbookAnalyzer::new();

    // Try to load manifest
    match crate::cli::runbooks::load_workspace_manifest_from_manifest_path(
        config.manifest_path.to_str().unwrap(),
    ) {
        Ok(manifest) => {
            let runbooks = workspace.find_all_runbooks_in_manifest(&manifest);

            if runbooks.is_empty() {
                if config.should_print_diagnostics() {
                    println!("No runbooks found in manifest.");
                }
                return Ok(());
            }

            let mut any_errors = false;
            for (name, location) in runbooks {
                if config.should_print_diagnostics() {
                    println!("Checking runbook '{}'...", name);
                }

                if analyze_runbook_with_manifest(&analyzer, &location, &manifest, config).is_err() {
                    any_errors = true;
                }

                if config.should_print_diagnostics() {
                    println!();
                }
            }

            if any_errors {
                Err("Doctor found errors in one or more runbooks".to_string())
            } else {
                Ok(())
            }
        }
        Err(_) => {
            // No manifest, try to find runbooks in current directory
            let runbooks = WorkspaceAnalyzer::find_runbooks_in_directory()?;

            let mut any_errors = false;
            for path in runbooks {
                if analyze_runbook_file(&analyzer, &path, config).is_err() {
                    any_errors = true;
                }
            }

            if any_errors {
                Err("Doctor found errors in one or more runbooks".to_string())
            } else {
                Ok(())
            }
        }
    }
}

/// Analyze a runbook file without manifest context
fn analyze_runbook_file(
    analyzer: &analyzer::RunbookAnalyzer,
    path: &Path,
    config: &DoctorConfig,
) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    let result = analyzer.analyze_runbook_with_context(path, &content, None, None, &[]);
    display_results(&result, &config.format);

    if result.errors.is_empty() {
        Ok(())
    } else {
        Err("Doctor found errors in your runbook".to_string())
    }
}

/// Analyze a runbook with manifest context
fn analyze_runbook_with_manifest(
    analyzer: &analyzer::RunbookAnalyzer,
    location: &RunbookLocation,
    manifest: &WorkspaceManifest,
    config: &DoctorConfig,
) -> Result<(), String> {
    // Use the executor's function to load the runbook (handles both files and directories)
    let file_location = FileLocation::from_path_string(&location.path.to_string_lossy())?;
    let (_, _, runbook_sources) = read_runbook_from_location(
        &file_location,
        &location.name.clone().into(),
        &config.environment,
        Some(&location.name),
    )?;

    // For multi-file runbooks, we need to combine all content
    if runbook_sources.tree.len() > 1 {
        // Combine content from all files and track boundaries
        let mut combined_content = String::new();
        let mut file_boundaries = Vec::new();
        let mut current_line = 1usize;

        for (file_location, (_name, raw_content)) in &runbook_sources.tree {
            let start_line = current_line;
            let content = raw_content.to_string();
            combined_content.push_str(&content);
            combined_content.push('\n');
            let line_count = content.lines().count();
            current_line += line_count + 1;
            file_boundaries.push((file_location.to_string(), start_line, current_line));
        }

        // Analyze the combined content
        let combined_path = location.path.join("_combined.tx");
        let result = analyzer.analyze_runbook_with_context(
            &combined_path,
            &combined_content,
            Some(manifest),
            config.environment.as_ref(),
            &config.cli_inputs,
        );

        // Map errors back to their original files
        let mut final_result = txtx_core::validation::ValidationResult {
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        };

        for error in result.errors {
            if let Some(line) = error.line {
                // Find which file this line belongs to
                for (file_path, start_line, end_line) in &file_boundaries {
                    if line >= *start_line && line < *end_line {
                        let mut mapped_error = error.clone();
                        mapped_error.file = file_path.clone();
                        mapped_error.line = Some(line - start_line);
                        final_result.errors.push(mapped_error);
                        break;
                    }
                }
            } else {
                final_result.errors.push(error);
            }
        }

        // Similar mapping for warnings
        for warning in result.warnings {
            if let Some(line) = warning.line {
                for (file_path, start_line, end_line) in &file_boundaries {
                    if line >= *start_line && line < *end_line {
                        let mut mapped_warning = warning.clone();
                        mapped_warning.file = file_path.clone();
                        mapped_warning.line = Some(line - start_line);
                        final_result.warnings.push(mapped_warning);
                        break;
                    }
                }
            } else {
                final_result.warnings.push(warning);
            }
        }

        final_result.suggestions = result.suggestions;

        display_results(&final_result, &config.format);

        if final_result.errors.is_empty() {
            Ok(())
        } else {
            Err("Doctor found errors in your runbook".to_string())
        }
    } else {
        // Single file - analyze normally
        let (source_location, (_, raw_content)) = runbook_sources.tree.iter().next().unwrap();
        let file_path = PathBuf::from(source_location.to_string());
        let content = raw_content.to_string();
        let result = analyzer.analyze_runbook_with_context(
            &file_path,
            &content,
            Some(manifest),
            config.environment.as_ref(),
            &config.cli_inputs,
        );

        display_results(&result, &config.format);

        if result.errors.is_empty() {
            Ok(())
        } else {
            Err("Doctor found errors in your runbook".to_string())
        }
    }
}

// Re-export for backward compatibility and LSP integration
pub use analyzer::rules::{
    CliInputOverrideRule, InputDefinedRule, InputNamingConventionRule, SensitiveDataRule,
};
pub use analyzer::{ValidationContext, ValidationOutcome, ValidationRule};
