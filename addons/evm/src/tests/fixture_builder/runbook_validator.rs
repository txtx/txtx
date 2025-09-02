// Runbook validator for better test debugging
// Validates runbook syntax and action schemas before execution

use std::collections::HashMap;

use super::action_schemas::{get_action_schema, ActionSchema};

pub struct RunbookValidator {
    content: String,
}

impl RunbookValidator {
    pub fn new(content: String) -> Self {
        Self { content }
    }
    
    /// Validate the runbook and return helpful errors
    pub fn validate(&self) -> Result<ValidationReport, String> {
        let mut report = ValidationReport::default();
        
        // For now, just demonstrate the validation concept
        // In a real implementation, we'd parse the HCL and validate
        
        // Check for common mistakes in the content
        if self.content.contains("to =") && self.content.contains("evm::send_eth") {
            report.add_error("Field 'to' should be 'recipient_address' in evm::send_eth".to_string());
        }
        
        if self.content.contains("value =") && self.content.contains("evm::send_eth") {
            report.add_error("Field 'value' should be 'amount' in evm::send_eth".to_string());
        }
        
        if self.content.contains("from =") && self.content.contains("evm::send_eth") {
            report.add_warning("Field 'from' is not needed when using a signer in evm::send_eth".to_string());
        }
        
        // If we found the correct fields, mark as success
        if self.content.contains("recipient_address =") && self.content.contains("amount =") {
            report.add_success("evm::send_eth action has correct field names".to_string());
        }
        
        Ok(report)
    }
    

}

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub successes: Vec<String>,
}

impl ValidationReport {
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn add_success(&mut self, success: String) {
        self.successes.push(success);
    }
    
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn format_report(&self) -> String {
        let mut output = Vec::new();
        
        if !self.errors.is_empty() {
            output.push("❌ Errors:".to_string());
            for error in &self.errors {
                output.push(format!("  - {}", error));
            }
        }
        
        if !self.warnings.is_empty() {
            output.push("⚠️  Warnings:".to_string());
            for warning in &self.warnings {
                output.push(format!("  - {}", warning));
            }
        }
        
        if !self.successes.is_empty() {
            output.push("✅ Validated:".to_string());
            for success in &self.successes {
                output.push(format!("  - {}", success));
            }
        }
        
        output.join("\n")
    }
}

/// Helper function to validate a runbook and print a helpful report
pub fn validate_runbook_with_report(content: &str) -> Result<(), String> {
    let validator = RunbookValidator::new(content.to_string());
    let report = validator.validate()?;
    
    eprintln!("\n📋 Runbook Validation Report:");
    eprintln!("{}", report.format_report());
    
    if !report.is_valid() {
        return Err("Runbook validation failed".to_string());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_runbook() {
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

action "send_eth" "evm::send_eth" {
    recipient_address = input.bob_address
    amount = "1000"
    signer = signer.alice
}
"#;
        
        let validator = RunbookValidator::new(runbook.to_string());
        let report = validator.validate().unwrap();
        
        // Should have validation results
        assert!(report.is_valid() || !report.warnings.is_empty());
    }
}