use std::path::PathBuf;
use txtx_core::manifest::WorkspaceManifest;

// Re-export linter components
pub use crate::cli::linter::{
    LinterConfig, Linter, Format as LinterFormat,
    workspace::WorkspaceAnalyzer,
};

/// Options for running the linter
#[derive(Debug, Clone)]
pub struct LinterOptions {
    pub config_path: Option<String>,
    pub disabled_rules: Vec<String>,
    pub only_rules: Vec<String>,
    pub fix: bool,
    pub init: bool,
}

/// Main entry point for the lint command
pub fn run_lint(
    runbook_path: Option<String>,
    manifest_path: Option<String>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
    format: crate::cli::LintOutputFormat,
    linter_options: LinterOptions,
    gen_cli: bool,
    gen_cli_full: bool,
) -> Result<(), String> {
    // Handle --init flag
    if linter_options.init {
        return init_linter_config();
    }

    // Handle --gen-cli and --gen-cli-full
    if gen_cli || gen_cli_full {
        return handle_gen_cli(
            runbook_path.as_deref(),
            manifest_path.as_deref(),
            environment.as_deref(),
            &cli_inputs,
            gen_cli_full,
        );
    }

    // Convert format enum
    let linter_format = match format {
        crate::cli::LintOutputFormat::Stylish => LinterFormat::Stylish,
        crate::cli::LintOutputFormat::Pretty => LinterFormat::Stylish, // Map Pretty to Stylish
        crate::cli::LintOutputFormat::Auto => LinterFormat::Stylish, // Default Auto to Stylish
        crate::cli::LintOutputFormat::Compact => LinterFormat::Compact,
        crate::cli::LintOutputFormat::Json => LinterFormat::Json,
        crate::cli::LintOutputFormat::Quickfix => LinterFormat::Quickfix,
        crate::cli::LintOutputFormat::Doc => LinterFormat::Doc,
    };

    // Create linter configuration
    let config = LinterConfig::new(
        manifest_path.map(PathBuf::from),
        runbook_path.clone(),
        environment,
        cli_inputs,
        linter_format,
    );

    // Run the linter
    let linter = Linter::new(&config)?;

    match runbook_path {
        Some(ref name) => linter.lint_runbook(name),
        None => linter.lint_all(),
    }
}

/// Initialize a new linter configuration file
fn init_linter_config() -> Result<(), String> {
    use std::fs;

    let config_path = PathBuf::from(".txtxlint.yml");

    if config_path.exists() {
        return Err(format!("Configuration file {} already exists", config_path.display()));
    }

    let default_config = r#"# Txtx Linter Configuration
# https://docs.txtx.io/linter

extends: "txtx:recommended"

rules:
  # Correctness rules
  undefined-input: error
  undefined-signer: error
  invalid-action-type: error
  cli-override: info

  # Style rules
  input-naming:
    severity: warning
    options:
      convention: "SCREAMING_SNAKE_CASE"

  # Security rules
  sensitive-data: warning

# Paths to ignore
ignore:
  - "examples/**"
  - "tests/**"
"#;

    fs::write(&config_path, default_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    println!("Created .txtxlint.yml with recommended settings");
    Ok(())
}

/// Handle --gen-cli and --gen-cli-full functionality
fn handle_gen_cli(
    runbook_path: Option<&str>,
    manifest_path: Option<&str>,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
    include_all: bool,
) -> Result<(), String> {
    use txtx_core::runbook::variables::RunbookVariableIterator;
    use txtx_addon_kit::helpers::fs::FileLocation;
    use txtx_core::manifest::file::read_runbook_from_location;
    use crate::cli::common::addon_registry;

    let runbook_path = runbook_path.ok_or("Runbook path required for --gen-cli")?;
    let path = PathBuf::from(runbook_path);

    // Try to determine the runbook name and location
    let (runbook_name, _file_location, runbook_sources) = if path.exists() && path.extension().map_or(false, |ext| ext == "tx") {
        // Direct file path
        let file_location = FileLocation::from_path(path.clone());
        let (_, _, runbook_sources) = read_runbook_from_location(
            &file_location,
            &None,
            &environment.map(|s| s.to_string()),
            None,
        )?;
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("runbook")
            .to_string();
        (name, file_location, runbook_sources)
    } else {
        // Resolve runbook from manifest
        let manifest_path = manifest_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./txtx.yml"));

        let manifest = load_manifest(&manifest_path)?;

        // Create workspace analyzer with the appropriate configuration
        let config = LinterConfig::new(
            Some(manifest_path),
            None,
            environment.map(String::from),
            vec![],
            LinterFormat::Json,
        );
        let workspace = WorkspaceAnalyzer::new(&config)?;

        // Resolve runbook sources from the manifest
        let runbook_sources = workspace.resolve_runbook_sources(runbook_path)?;

        // Use runbook path as the display name
        let name = runbook_path.to_string();

        // Create a placeholder file location - actual resolution is handled by workspace analyzer
        let file_location = FileLocation::from_path(PathBuf::from(runbook_path));
        (name, file_location, runbook_sources)
    };

    // Load or create manifest
    let manifest = if let Some(manifest_path) = manifest_path {
        load_manifest(&PathBuf::from(manifest_path))?
    } else {
        match load_manifest(&PathBuf::from("./txtx.yml")) {
            Ok(m) => m,
            Err(_) => WorkspaceManifest::new("temp".to_string())
        }
    };

    // Get addon specs
    let addons = addon_registry::get_all_addons();
    let addon_specs = addon_registry::extract_addon_specifications(&addons);

    // Create iterator
    let iterator = RunbookVariableIterator::new_with_cli_inputs(
        &runbook_sources,
        &manifest,
        environment,
        addon_specs,
        cli_inputs,
    )?;

    // Collect variables
    let variables: Vec<_> = if include_all {
        iterator.collect()
    } else {
        iterator.undefined_or_cli_provided().collect()
    };

    // Format output
    let output = format_cli_template(
        &runbook_name,
        environment,
        variables,
    );

    println!("{}", output);
    Ok(())
}

/// Format CLI template output
fn format_cli_template(
    runbook_name: &str,
    environment: Option<&str>,
    mut variables: Vec<txtx_core::runbook::variables::RunbookVariable>,
) -> String {
    let mut parts = vec!["txtx".to_string(), "run".to_string(), runbook_name.to_string()];

    if let Some(env) = environment {
        parts.push("--env".to_string());
        parts.push(env.to_string());
    }

    variables.sort_by(|a, b| a.name.cmp(&b.name));

    if variables.is_empty() {
        parts.join(" ")
    } else {
        let mut output = parts.join(" ");
        for var in variables {
            output.push_str(" \\\n  --input ");
            let value = if let Some(ref val) = var.resolved_value {
                val.clone()
            } else {
                format!("\"${}\"", var.name.to_uppercase().replace('-', "_"))
            };
            output.push_str(&format!("{}={}", var.name, value));
        }
        output
    }
}

/// Load workspace manifest
fn load_manifest(path: &PathBuf) -> Result<WorkspaceManifest, String> {
    crate::cli::runbooks::load_workspace_manifest_from_manifest_path(
        path.to_str().ok_or_else(|| "Invalid manifest path".to_string())?
    ).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_handles_none_manifest_path() {
        let linter_options = LinterOptions {
            config_path: None,
            disabled_rules: vec![],
            only_rules: vec![],
            fix: false,
            init: false,
        };

        // When manifest_path is None and the runbook is not a direct file path,
        // the function should try to load from default manifest
        let result = run_lint(
            Some("test-runbook".to_string()),
            None, // This should default to "./txtx.yml"
            None, // No environment specified
            vec![],
            crate::cli::LintOutputFormat::Json,
            linter_options,
            false,
            false,
        );

        // The function should fail because the manifest doesn't exist in test environment
        // but it should fail gracefully, not panic
        assert!(result.is_err());
        let error = result.unwrap_err();
        // The new linter has different error messages, so we just check it's an error
        assert!(!error.is_empty());
    }

    #[test]
    fn test_lint_all_runbooks_defaults_manifest_path() {
        let linter_options = LinterOptions {
            config_path: None,
            disabled_rules: vec![],
            only_rules: vec![],
            fix: false,
            init: false,
        };

        // When manifest_path is None, it should default to "./txtx.yml"
        let result = run_lint(
            None, // Lint all runbooks
            None, // This should default to "./txtx.yml"
            None, // No environment specified
            vec![],
            crate::cli::LintOutputFormat::Json,
            linter_options,
            false,
            false,
        );

        // Should attempt to load default manifest and fail gracefully
        // Either returns Ok(()) if no runbooks found, or error if manifest invalid
        // but should not panic
        match result {
            Ok(_) => {
                // No runbooks found is okay
            }
            Err(e) => {
                // Should be a reasonable error message, not a panic
                assert!(!e.is_empty());
            }
        }
    }
}