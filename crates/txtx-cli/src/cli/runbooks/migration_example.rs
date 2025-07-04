/// Migration examples showing how to update CLI error handling from String/Diagnostic to error-stack

use error_stack::{Report, ResultExt};
use crate::cli::errors::{CliError, IntoCliError, CliErrorExt};
use txtx_addon_kit::types::errors::{TxtxError, ErrorAttachments};
use txtx_core::manifest::WorkspaceManifest;
use txtx_addon_kit::helpers::fs::FileLocation;

// ============================================================================
// BEFORE: Using String errors
// ============================================================================

pub fn load_workspace_manifest_old(manifest_path: &str) -> Result<WorkspaceManifest, String> {
    let manifest_location = FileLocation::from_path_string(manifest_path)?;
    WorkspaceManifest::from_location(&manifest_location)
}

pub fn load_runbook_old(
    manifest_path: &str,
    runbook_name: &str,
) -> Result<String, String> {
    let manifest = load_workspace_manifest_old(manifest_path)?;
    
    // Find runbook
    if !manifest.runbooks.contains_key(runbook_name) {
        return Err(format!(
            "unable to retrieve runbook '{}' in manifest", 
            runbook_name
        ));
    }
    
    Ok(runbook_name.to_string())
}

// ============================================================================
// AFTER: Using error-stack
// ============================================================================

pub fn load_workspace_manifest_new(manifest_path: &str) -> Result<WorkspaceManifest, Report<CliError>> {
    let manifest_location = FileLocation::from_path_string(manifest_path)
        .into_cli_error(CliError::ManifestError)?;
    
    WorkspaceManifest::from_location(&manifest_location)
        .map_err(|e| Report::new(CliError::ManifestError)
            .attach_printable(format!("Failed to load manifest: {}", e)))
        .with_manifest_info(manifest_path, "TOML")
        .with_documentation("Ensure Txtx.toml exists and is valid TOML")
        .with_example("txtx run deploy --manifest ./Txtx.toml")
}

pub fn load_runbook_new(
    manifest_path: &str,
    runbook_name: &str,
    environment: Option<String>,
) -> Result<String, Report<CliError>> {
    let manifest = load_workspace_manifest_new(manifest_path)?;
    
    // Find runbook with rich error context
    if !manifest.runbooks.contains_key(runbook_name) {
        let available_runbooks: Vec<String> = manifest.runbooks.keys().cloned().collect();
        
        return Err(Report::new(CliError::RunbookNotFound))
            .attach_printable(format!(
                "Runbook '{}' not found in manifest", 
                runbook_name
            ))
            .attach_printable(format!(
                "Available runbooks: {}", 
                available_runbooks.join(", ")
            ))
            .with_runbook_context(runbook_name, manifest_path, environment)
            .with_documentation("Check the runbook name matches one defined in your manifest")
            .with_example(format!("txtx run {}", 
                available_runbooks.first().unwrap_or(&"example".to_string())
            ));
    }
    
    Ok(runbook_name.to_string())
}

// ============================================================================
// More migration examples
// ============================================================================

/// Before: Generic auth error
pub fn check_auth_old() -> Result<(), String> {
    // Simulated auth check
    let is_authenticated = false;
    
    if !is_authenticated {
        return Err("Runbook contains cloud service actions, but you are not authenticated.\nRun the command `txtx cloud login` to log in.".to_string());
    }
    Ok(())
}

/// After: Rich auth error with context
pub fn check_auth_new() -> Result<(), Report<CliError>> {
    // Simulated auth check
    let is_authenticated = false;
    let required_service = "txtx-cloud";
    
    if !is_authenticated {
        return Err(Report::new(CliError::AuthError))
            .attach_printable("Authentication required for cloud service actions")
            .attach_printable(format!("Service: {}", required_service))
            .with_documentation("You must be authenticated to use cloud services")
            .with_example("txtx cloud login")
            .with_link("https://docs.txtx.io/cloud/authentication");
    }
    Ok(())
}

/// Before: Environment variable error
pub fn get_env_var_old(key: &str) -> Result<String, String> {
    std::env::var(key)
        .map_err(|_| format!("Missing required environment variable: {}", key))
}

/// After: Environment error with suggestions
pub fn get_env_var_new(key: &str) -> Result<String, Report<CliError>> {
    std::env::var(key)
        .map_err(|_| Report::new(CliError::EnvironmentError)
            .attach_printable(format!("Missing required environment variable: {}", key)))
        .with_documentation(match key {
            "INFURA_API_KEY" => "Get your API key from https://infura.io",
            "ETHERSCAN_API_KEY" => "Get your API key from https://etherscan.io/apis",
            _ => "Set this environment variable in your .env file or shell",
        })
        .with_example(format!("export {}=your_value_here", key))
}

/// Before: Output write error
pub fn write_output_old(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write output to {}: {}", path, e))
}

/// After: Output error with recovery suggestions
pub fn write_output_new(path: &str, content: &str) -> Result<(), Report<CliError>> {
    std::fs::write(path, content)
        .map_err(|e| Report::new(CliError::OutputError)
            .attach_printable(format!("IO Error: {}", e)))
        .with_output_info(path, "JSON", e.to_string())
        .with_documentation("Check that the directory exists and you have write permissions")
        .with_example("txtx run deploy --output ./outputs/result.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runbook_not_found_migration() {
        // Old error
        let old_result = load_runbook_old("./Txtx.toml", "nonexistent");
        assert!(old_result.is_err());
        let old_error = old_result.unwrap_err();
        assert!(old_error.contains("unable to retrieve runbook"));
        
        // New error - would need mock manifest for full test
        // Just demonstrating the pattern
    }

    #[test]
    fn test_auth_error_migration() {
        // Old
        let old_result = check_auth_old();
        assert!(old_result.is_err());
        
        // New
        let new_result = check_auth_new();
        assert!(new_result.is_err());
        let error = new_result.unwrap_err();
        
        // New error has rich context
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Authentication required"));
        assert!(error_string.contains("txtx cloud login"));
    }
}