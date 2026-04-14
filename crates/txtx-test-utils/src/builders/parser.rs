use txtx_addon_kit::hcl::structure::Block;
use txtx_addon_kit::helpers::hcl::RawHclContent;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::construct_type::ConstructType;

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
        .filter(|b| b.block_type == ConstructType::Signer)
        .filter_map(|b| b.labels.first().cloned())
        .collect()
}

/// Extract actions from parsed blocks
pub fn extract_actions(blocks: &[ParsedBlock]) -> Vec<String> {
    blocks
        .iter()
        .filter(|b| b.block_type == ConstructType::Action)
        .filter_map(|b| b.labels.first().cloned())
        .collect()
}

/// Scan `content` for occurrences of any prefix in `prefixes` followed by an
/// identifier (alphanumeric + `_`), returning the deduplicated, sorted list of
/// identifiers found.
fn extract_prefixed_idents(content: &str, prefixes: &[&str]) -> Vec<String> {
    let mut references = Vec::new();

    for pattern in prefixes {
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find(pattern) {
            let start = search_from + pos + pattern.len();
            let rest = &content[start..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());

            if end > 0 {
                let ident = &rest[..end];
                if !ident.is_empty() {
                    references.push(ident.to_string());
                }
            }

            search_from = start + end;
        }
    }

    references.sort();
    references.dedup();
    references
}

/// Find references to signers in content (e.g., `signer.my_key`, `signers.my_key`).
pub fn find_signer_references(content: &str) -> Vec<String> {
    extract_prefixed_idents(content, &["signer.", "signers."])
}

/// Find references to actions in content (e.g., `action.deploy`).
pub fn find_action_references(content: &str) -> Vec<String> {
    extract_prefixed_idents(content, &["action."])
}

/// Find environment variable references in content (e.g., `env.API_KEY`).
pub fn find_env_references(content: &str) -> Vec<String> {
    extract_prefixed_idents(content, &["env."])
}
