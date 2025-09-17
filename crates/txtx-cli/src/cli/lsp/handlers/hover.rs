//! Hover information handler
//!
//! Provides hover information for functions, actions, and input references

use super::{Handler, TextDocumentHandler};
use super::debug_dump::DebugDumpHandler;
use super::environment_resolver::EnvironmentResolver;
use crate::cli::lsp::{
    functions::{get_action_hover, get_function_hover, get_signer_hover},
    utils::environment,
    workspace::SharedWorkspaceState,
};
use lsp_types::{*, Url};

pub struct HoverHandler {
    workspace: SharedWorkspaceState,
    debug_handler: DebugDumpHandler,
}

impl HoverHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        let debug_handler = DebugDumpHandler::new(workspace.clone());
        Self { 
            workspace,
            debug_handler,
        }
    }

    /// Handle hover request
    pub fn hover(&self, params: HoverParams) -> Option<Hover> {
        let (uri, content, position) =
            self.get_document_at_position(&params.text_document_position_params)?;

        eprintln!("[HOVER DEBUG] Position: line {}, char {}", position.line, position.character);

        // Try to extract function/action reference
        if let Some(hover) = self.try_function_or_action_hover(&content, &position, &uri) {
            return Some(hover);
        }

        // Try input reference hover
        if let Some(hover) = self.try_input_hover(&content, &position, &uri) {
            return Some(hover);
        }

        eprintln!("[HOVER DEBUG] No hover information found at position");
        None
    }

    /// Try to provide hover for function, action, or signer references
    fn try_function_or_action_hover(&self, content: &str, position: &Position, uri: &Url) -> Option<Hover> {
        let reference = extract_function_or_action(content, position)?;
        eprintln!("[HOVER DEBUG] Extracted function/action reference: '{}'", reference);
        
        // Check if it's a function
        if let Some(hover_text) = get_function_hover(&reference) {
            eprintln!("[HOVER DEBUG] Resolved as function");
            return Some(self.create_markdown_hover(hover_text));
        }

        // Check if it's an action
        if let Some(hover_text) = get_action_hover(&reference) {
            eprintln!("[HOVER DEBUG] Resolved as action");
            return Some(self.create_markdown_hover(hover_text));
        }

        // Check if it's a signer
        if let Some(hover) = self.try_signer_hover(&reference, uri) {
            return Some(hover);
        }

        eprintln!("[HOVER DEBUG] Reference '{}' not resolved as function/action/signer", reference);
        None
    }

    /// Try to provide hover for signer references
    fn try_signer_hover(&self, reference: &str, uri: &Url) -> Option<Hover> {
        // First check for static signers from addons
        if let Some(hover_text) = get_signer_hover(reference) {
            eprintln!("[HOVER DEBUG] Resolved as signer from addon");
            return Some(self.create_markdown_hover(hover_text));
        }
        
        // If not found in static signers, check environment-specific signers
        let workspace = self.workspace.read();
        let current_env = workspace.get_current_environment()
            .or_else(|| environment::extract_environment_from_uri(uri))
            .unwrap_or_else(|| "global".to_string());
        
        eprintln!("[HOVER DEBUG] Checking for signer '{}' in environment '{}'", reference, current_env);
        
        // Check if it's a namespace::signer pattern
        if reference.contains("::") {
            let parts: Vec<&str> = reference.split("::").collect();
            if parts.len() == 2 {
                let namespace = parts[0];
                let signer_name = parts[1];
                
                // Provide a generic hover text for environment-specific signers
                let hover_text = format!(
                    "### Signer: `{}`\n\n\
                    **Namespace**: `{}`\n\
                    **Environment**: `{}`\n\n\
                    This signer may be defined in an environment-specific file.\n\n\
                    💡 **Tip**: Check `*.{}.tx` files for environment-specific signer definitions.",
                    signer_name, namespace, current_env, current_env
                );
                
                eprintln!("[HOVER DEBUG] Providing generic hover for environment signer");
                return Some(self.create_markdown_hover(hover_text));
            }
        }

        None
    }

    /// Try to provide hover for input references
    fn try_input_hover(&self, content: &str, position: &Position, uri: &Url) -> Option<Hover> {
        let var_ref = extract_input_reference(content, position)?;
        eprintln!("[HOVER DEBUG] Extracted input reference: 'input.{}'", var_ref);
        
        // Special debug commands
        if var_ref == "dump_txtx_state" {
            eprintln!("[HOVER DEBUG] Resolved as special debug command: dump_txtx_state");
            return self.debug_handler.dump_state(uri);
        }

        if var_ref.starts_with("dump_txtx_var_") {
            let variable_name = &var_ref["dump_txtx_var_".len()..];
            eprintln!("[HOVER DEBUG] Resolved as special debug command: dump_txtx_var_{}", variable_name);
            return self.debug_handler.dump_variable(uri, variable_name);
        }

        // Regular input variable hover
        self.create_input_hover(uri, &var_ref)
    }

    /// Create hover information for an input variable
    fn create_input_hover(&self, uri: &Url, var_ref: &str) -> Option<Hover> {
        let workspace = self.workspace.read();

        // Get the current environment
        let current_env = workspace.get_current_environment()
            .or_else(|| environment::extract_environment_from_uri(uri))
            .unwrap_or_else(|| "global".to_string());

        eprintln!("[HOVER DEBUG] Current environment: '{}'", current_env);

        // Get manifest for the document
        let manifest = workspace.get_manifest_for_document(uri)?;
        let resolver = EnvironmentResolver::new(&manifest, current_env.clone());

        let mut hover_text = format!("**Input**: `{}`\n\n", var_ref);

        // Try to resolve the value in current environment
        if let Some((value, source)) = resolver.resolve_value(var_ref) {
            // Input is available
            hover_text.push_str(&format!("**Current value**: `{}`\n", value));
            hover_text.push_str(&format!("**Environment**: `{}`", current_env));

            if source == "global" && current_env != "global" {
                hover_text.push_str(" *(inherited from global)*");
            }
            hover_text.push_str("\n\n");

            // Show other environments where it's defined
            let all_values = resolver.get_all_values(var_ref);
            if all_values.len() > 1 {
                hover_text.push_str("**Also defined in:**\n");
                for (env_name, env_value) in &all_values {
                    if env_name != &current_env && !(source == "global" && env_name == "global") {
                        hover_text.push_str(&format!("- `{}`: `{}`\n", env_name, env_value));
                    }
                }
            }
        } else {
            // Input not available in current environment
            let all_values = resolver.get_all_values(var_ref);
            
            if !all_values.is_empty() {
                // Available elsewhere
                hover_text.push_str(&format!(
                    "⚠️ **Not available** in environment `{}`\n\n",
                    current_env
                ));
                hover_text.push_str("**Available in:**\n");
                for (env_name, env_value) in &all_values {
                    hover_text.push_str(&format!("- `{}`: `{}`\n", env_name, env_value));
                }
                hover_text.push_str(&format!(
                    "\n💡 Switch to one of these environments or add this input to `{}`",
                    current_env
                ));
            } else {
                // Not found anywhere
                hover_text.push_str("⚠️ **Not defined** in any environment\n\n");
                hover_text.push_str(
                    "Add this input to your `txtx.yml` file:\n```yaml\nenvironments:\n  ",
                );
                hover_text.push_str(&current_env);
                hover_text.push_str(&format!(":\n    {}: \"<value>\"\n```", var_ref));
            }
        }

        eprintln!("[HOVER DEBUG] Returning hover text for input '{}'", var_ref);
        Some(self.create_markdown_hover(hover_text))
    }

    /// Create a hover response with markdown content
    fn create_markdown_hover(&self, content: String) -> Hover {
        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: None,
        }
    }
}

impl Handler for HoverHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl TextDocumentHandler for HoverHandler {}

// Helper function to check if a position is within a comment
fn is_in_comment(content: &str, position: &Position) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    if let Some(line) = lines.get(position.line as usize) {
        // Check for line comments starting with //
        if let Some(comment_start) = line.find("//") {
            if position.character >= comment_start as u32 {
                return true;
            }
        }
        
        // Check for line comments starting with #
        if let Some(comment_start) = line.find('#') {
            // Make sure it's not inside a string
            // Simple heuristic: count quotes before the #
            let before_hash = &line[..comment_start];
            let quote_count = before_hash.chars().filter(|c| *c == '"').count();
            
            // If even number of quotes, we're likely not in a string
            if quote_count % 2 == 0 && position.character >= comment_start as u32 {
                return true;
            }
        }
        
        // TODO: Handle block comments /* */ if HCL supports them
    }
    false
}

fn extract_function_or_action(content: &str, position: &Position) -> Option<String> {
    // Skip if position is in a comment
    if is_in_comment(content, position) {
        return None;
    }
    
    let lines: Vec<&str> = content.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Simple heuristic: look for namespace::name pattern
    let re = regex::Regex::new(r"\b(\w+)::([\w_]+)\b").ok()?;

    for capture in re.captures_iter(line) {
        let full_match = capture.get(0)?;
        let start = full_match.start() as u32;
        let end = full_match.end() as u32;

        if position.character >= start && position.character <= end {
            return Some(full_match.as_str().to_string());
        }
    }

    None
}

fn extract_input_reference(content: &str, position: &Position) -> Option<String> {
    // Skip if position is in a comment
    if is_in_comment(content, position) {
        return None;
    }
    
    let lines: Vec<&str> = content.lines().collect();
    let line = lines.get(position.line as usize)?;

    // Look for input.variable_name pattern
    let re = regex::Regex::new(r"input\.(\w+)").ok()?;

    for capture in re.captures_iter(line) {
        if let Some(var_match) = capture.get(1) {
            let full_match = capture.get(0)?;
            let start = full_match.start() as u32;
            let end = full_match.end() as u32;

            // Check if cursor position is within the match bounds
            if position.character >= start && position.character < end {
                return Some(var_match.as_str().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_comment() {
        // Test regular code - not in comment
        let content = "value = std::encode_hex(data)";
        let position = Position { line: 0, character: 15 };
        assert_eq!(is_in_comment(content, &position), false);

        // Test // comment
        let content = "// This is a comment";
        let position = Position { line: 0, character: 10 };
        assert_eq!(is_in_comment(content, &position), true);

        // Test # comment
        let content = "# This is a comment";
        let position = Position { line: 0, character: 10 };
        assert_eq!(is_in_comment(content, &position), true);

        // Test code before comment
        let content = "value = 5 // comment";
        let position = Position { line: 0, character: 5 };
        assert_eq!(is_in_comment(content, &position), false);

        // Test position in comment after code
        let content = "value = 5 // comment";
        let position = Position { line: 0, character: 15 };
        assert_eq!(is_in_comment(content, &position), true);
    }

    #[test]
    fn test_extract_function_reference() {
        let content = "value = std::encode_hex(data)";
        let position = Position { line: 0, character: 15 };

        // Debug: check if incorrectly detected as comment
        assert_eq!(is_in_comment(content, &position), false, "Should not be detected as comment");

        let result = extract_function_or_action(content, &position);
        assert_eq!(result, Some("std::encode_hex".to_string()));
    }

    #[test]
    fn test_extract_action_reference() {
        let content = "action \"deploy\" \"evm::deploy_contract\" {";
        let position = Position { line: 0, character: 20 };

        // Debug: check if incorrectly detected as comment
        assert_eq!(is_in_comment(content, &position), false, "Should not be detected as comment");

        let result = extract_function_or_action(content, &position);
        assert_eq!(result, Some("evm::deploy_contract".to_string()));
    }

    #[test]
    fn test_extract_input_reference() {
        let content = "value = input.api_key";
        let position = Position { line: 0, character: 15 };

        let result = extract_input_reference(content, &position);
        assert_eq!(result, Some("api_key".to_string()));
    }

    #[test]
    fn test_extract_input_dump_txtx_state() {
        let content = "debug = input.dump_txtx_state";
        
        // The string "input.dump_txtx_state" starts at position 8
        // Test hovering at 'i' of input (position 8)
        let position = Position { line: 0, character: 8 };
        let result = extract_input_reference(content, &position);
        assert_eq!(result, Some("dump_txtx_state".to_string()));
        
        // Test hovering at 'd' of dump (position 14)
        let position = Position { line: 0, character: 14 };
        let result = extract_input_reference(content, &position);
        assert_eq!(result, Some("dump_txtx_state".to_string()));
        
        // Test hovering in middle of "dump_txtx_state" (position 20)
        let position = Position { line: 0, character: 20 };
        let result = extract_input_reference(content, &position);
        assert_eq!(result, Some("dump_txtx_state".to_string()));
        
        // Test hovering at last character 'e' (position 28)
        let position = Position { line: 0, character: 28 };
        let result = extract_input_reference(content, &position);
        assert_eq!(result, Some("dump_txtx_state".to_string()));
        
        // Test hovering just after the match should return None
        let position = Position { line: 0, character: 29 };
        let result = extract_input_reference(content, &position);
        assert_eq!(result, None);
    }
}