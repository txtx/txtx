use txtx_addon_kit::hcl::structure::Block;
use txtx_addon_kit::helpers::hcl::RawHclContent;
use txtx_addon_kit::types::diagnostics::Diagnostic;

/// Parsed block information for validation
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub block_type: String,
    pub labels: Vec<String>,
    pub block: Block,
}

/// Parse HCL content into blocks for validation
pub fn parse_runbook_content(content: &str) -> Result<Vec<ParsedBlock>, Diagnostic> {
    let raw_content = RawHclContent::from_string(content.to_string());
    let mut blocks = raw_content.into_blocks()?;

    let mut parsed_blocks = Vec::new();

    while let Some(block) = blocks.pop_front() {
        let block_type = block.ident.value().to_string();
        let labels = block.labels.iter().map(|label| label.to_string()).collect();

        parsed_blocks.push(ParsedBlock { block_type, labels, block });
    }

    Ok(parsed_blocks)
}

/// Extract signers from parsed blocks
pub fn extract_signers(blocks: &[ParsedBlock]) -> Vec<String> {
    blocks
        .iter()
        .filter(|b| b.block_type == "signer")
        .filter_map(|b| b.labels.first().cloned())
        .collect()
}

/// Extract actions from parsed blocks
pub fn extract_actions(blocks: &[ParsedBlock]) -> Vec<String> {
    blocks
        .iter()
        .filter(|b| b.block_type == "action")
        .filter_map(|b| b.labels.first().cloned())
        .collect()
}

/// Find references to signers in content
pub fn find_signer_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    // Simple regex-like pattern matching for signer.xxx
    let patterns = ["signer.", "signers."];
    for pattern in &patterns {
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find(pattern) {
            let start = search_from + pos + pattern.len();

            // Find the end of the identifier
            let rest = &content[start..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());

            if end > 0 {
                let signer_name = &rest[..end];
                if !signer_name.is_empty() {
                    references.push(signer_name.to_string());
                }
            }

            search_from = start + end;
        }
    }

    references.sort();
    references.dedup();
    references
}

/// Find references to actions in content
pub fn find_action_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    // Simple pattern matching for action.xxx
    let pattern = "action.";
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(pattern) {
        let start = search_from + pos + pattern.len();

        // Find the action name (first identifier)
        let rest = &content[start..];
        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());

        if end > 0 {
            let action_name = &rest[..end];
            if !action_name.is_empty() {
                references.push(action_name.to_string());
            }
        }

        search_from = start + end;
    }

    references.sort();
    references.dedup();
    references
}

/// Find all environment variable references in the content (e.g., env.API_KEY)
pub fn find_env_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    // Simple pattern matching for env.xxx
    let pattern = "env.";
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(pattern) {
        let start = search_from + pos + pattern.len();

        // Find the env var name (identifier)
        let rest = &content[start..];
        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());

        if end > 0 {
            let env_var = &rest[..end];
            if !env_var.is_empty() {
                references.push(env_var.to_string());
            }
        }

        search_from = start + end;
    }

    references.sort();
    references.dedup();
    references
}
