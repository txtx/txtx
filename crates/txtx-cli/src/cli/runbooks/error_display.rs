use error_stack::Report;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::errors::{TxtxError, ErrorLocation, ErrorDocumentation, ActionContext};
use crate::cli::errors::{CliError, StateFileInfo, OutputInfo, CliErrorExt};
use txtx_core::types::{Runbook, RunbookStateLocation};
use txtx_core::utils::try_write_outputs_to_file;
use std::collections::HashMap;

/// Enhanced error display for runbook execution using error-stack
pub fn process_runbook_execution_output_v2(
    execution_result: Result<(), Vec<Report<TxtxError>>>,
    runbook: &mut Runbook,
    runbook_state_location: Option<RunbookStateLocation>,
    output_json: &Option<Option<String>>,
    output_filter: &Option<String>,
) {
    match execution_result {
        Err(errors) => handle_execution_errors(errors, runbook, runbook_state_location),
        Ok(()) => handle_successful_execution(runbook, output_json, output_filter),
    }
}

/// Handle and display execution errors with rich context
fn handle_execution_errors(
    errors: Vec<Report<TxtxError>>,
    runbook: &mut Runbook,
    runbook_state_location: Option<RunbookStateLocation>,
) {
    println!("\n{} Execution failed with {} error{}:", 
        red!("✗"), 
        errors.len(), 
        if errors.len() == 1 { "" } else { "s" }
    );
    
    for (idx, error) in errors.iter().enumerate() {
        println!("\n{} Error {}:", red!("▶"), idx + 1);
        display_error_with_context(error);
    }
    
    // Save transient state for recovery
    match runbook.mark_failed_and_write_transient_state(runbook_state_location.clone()) {
        Ok(Some(location)) => {
            println!("\n{} Transient state saved to: {}", yellow!("💾"), location);
            println!("   You can resume from this state using: txtx run --resume");
        }
        Ok(None) => {}
        Err(e) => {
            let state_error = Report::new(CliError::StateError)
                .attach_printable(format!("Failed to save transient state: {}", e));
            
            if let Some(location) = runbook_state_location {
                let _ = state_error.with_state_file_info(
                    location.to_string(),
                    "write transient state"
                );
            }
            
            println!("\n{} Warning: {}", yellow!("⚠"), "Could not save transient state");
            println!("   {}", e);
        }
    };
}

/// Display a single error with all its context and attachments
fn display_error_with_context(error: &Report<TxtxError>) {
    // Main error display - error-stack provides this formatting
    println!("{:?}", error);
    
    // Extract and display additional context
    if let Some(location) = error.downcast_ref::<ErrorLocation>() {
        println!("\n   📍 Location: {}:{}:{}", 
            location.file, 
            location.line, 
            location.column
        );
    }
    
    if let Some(action) = error.downcast_ref::<ActionContext>() {
        println!("\n   🎯 Action Details:");
        println!("      • Name: {}", action.action_name);
        println!("      • Type: {}::{}", action.namespace, action.construct_id);
    }
    
    if let Some(docs) = error.downcast_ref::<ErrorDocumentation>() {
        println!("\n   📚 Help:");
        println!("      {}", docs.help);
        
        if let Some(example) = &docs.example {
            println!("\n   💡 Example:");
            println!("      {}", example);
        }
        
        if let Some(link) = &docs.link {
            println!("\n   🔗 More info: {}", link);
        }
    }
    
    // Suggest recovery actions based on error type
    suggest_recovery_actions(error);
}

/// Suggest recovery actions based on the error type
fn suggest_recovery_actions(error: &Report<TxtxError>) {
    println!("\n   🔧 Suggested Actions:");
    
    // Get the root error context
    let error_string = format!("{:?}", error);
    
    if error_string.contains("Network") || error_string.contains("timeout") {
        println!("      • Check your network connection");
        println!("      • Verify RPC endpoint is accessible");
        println!("      • Try increasing timeout values");
        println!("      • Consider using a different RPC endpoint");
    } else if error_string.contains("Insufficient funds") {
        println!("      • Check account balance");
        println!("      • Ensure you have enough funds for gas + value");
        println!("      • Try reducing gas price or transaction value");
    } else if error_string.contains("Missing") || error_string.contains("not found") {
        println!("      • Verify all required inputs are provided");
        println!("      • Check your environment variables");
        println!("      • Review the runbook documentation");
    } else if error_string.contains("Type mismatch") {
        println!("      • Check the expected type in the error message");
        println!("      • Review your input values");
        println!("      • Ensure quotes are used correctly for strings");
    } else {
        println!("      • Review the error details above");
        println!("      • Check the documentation for this action");
        println!("      • Run with --verbose for more details");
    }
}

/// Handle successful execution and output writing
fn handle_successful_execution(
    runbook: &Runbook,
    output_json: &Option<Option<String>>,
    output_filter: &Option<String>,
) {
    let runbook_outputs = runbook.collect_formatted_outputs();

    if !runbook_outputs.is_empty() {
        if let Some(some_output_loc) = output_json {
            if let Some(output_loc) = some_output_loc {
                match write_outputs_with_error_context(
                    output_loc,
                    runbook_outputs.clone(),
                    &runbook.runtime_context.authorization_context.workspace_location,
                    &runbook.runbook_id.name,
                ) {
                    Ok(actual_path) => {
                        println!("{} Outputs written to {}", green!("✓"), actual_path);
                    }
                    Err(e) => {
                        println!("{} Failed to write outputs:", red!("✗"));
                        display_error_with_context(&e);
                        // Fall back to printing outputs
                        print_outputs_to_console(&runbook_outputs, output_filter);
                    }
                }
            } else {
                // Write to stdout (for piping)
                match serde_json::to_string(&runbook_outputs) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        let error = Report::new(CliError::OutputError)
                            .attach_printable(format!("Failed to serialize outputs: {}", e));
                        display_error_with_context(&error);
                    }
                }
            }
        } else {
            print_outputs_to_console(&runbook_outputs, output_filter);
        }
    }

    println!("\n{} Runbook completed successfully!", green!("✓"));
}

/// Write outputs with enhanced error context
fn write_outputs_with_error_context(
    output_loc: &str,
    outputs: HashMap<String, serde_json::Value>,
    workspace_location: &txtx_addon_kit::helpers::fs::FileLocation,
    runbook_name: &str,
) -> Result<String, Report<CliError>> {
    try_write_outputs_to_file(output_loc, outputs, workspace_location, runbook_name)
        .map_err(|e| Report::new(CliError::OutputError)
            .attach_printable(e))
        .with_output_info(
            output_loc,
            "JSON",
            "Check file permissions and path validity"
        )
}

/// Print outputs to console with optional filtering
fn print_outputs_to_console(
    outputs: &HashMap<String, serde_json::Value>,
    filter: &Option<String>,
) {
    println!("\n{} Outputs:", green!("📤"));
    
    let filtered_outputs: HashMap<String, serde_json::Value> = match filter {
        Some(f) => outputs
            .iter()
            .filter(|(k, _)| k.contains(f))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        None => outputs.clone(),
    };

    if filtered_outputs.is_empty() && filter.is_some() {
        println!("   No outputs match filter '{}'", filter.as_ref().unwrap());
        return;
    }

    for (key, value) in filtered_outputs.iter() {
        println!("   {} = {}", cyan!(key), value);
    }
}

/// Compatibility wrapper to migrate from Vec<Diagnostic> to Vec<Report<TxtxError>>
pub fn convert_diagnostics_to_reports(diagnostics: Vec<Diagnostic>) -> Vec<Report<TxtxError>> {
    diagnostics.into_iter()
        .map(|diag| diag.into())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use txtx_addon_kit::types::diagnostics::{Diagnostic, DiagnosticLevel};

    #[test]
    fn test_diagnostic_conversion() {
        let diag = Diagnostic::error_from_string("Test error".to_string());
        let reports = convert_diagnostics_to_reports(vec![diag]);
        
        assert_eq!(reports.len(), 1);
        let error_string = format!("{:?}", reports[0]);
        assert!(error_string.contains("Test error"));
    }
}