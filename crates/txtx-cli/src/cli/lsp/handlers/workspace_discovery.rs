//! Workspace file discovery utilities for LSP handlers
//!
//! Provides functions to discover all workspace files (manifest and runbooks)
//! for operations that need to search across the entire workspace, such as
//! find-all-references and rename.

use crate::cli::lsp::utils::file_scanner;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::{Position, Range, Url};
use regex::Regex;

/// Files discovered in the workspace for searching
#[derive(Debug)]
pub struct DiscoveredFiles {
    /// URI of the manifest file (txtx.yml/txtx.yaml), if found
    pub manifest_uri: Option<Url>,
    /// URIs of all runbooks listed in the manifest
    pub runbook_uris: Vec<Url>,
}

/// Discovers all workspace files by finding the manifest and extracting runbook URIs.
///
/// This function searches for the manifest file in two ways:
/// 1. First checks if the manifest is already open in the workspace
/// 2. If not found, walks up the directory tree from any open document
///
/// Once the manifest is found, it extracts all runbook references from it.
///
/// # Returns
///
/// A `DiscoveredFiles` struct containing:
/// - `manifest_uri`: The URI of the manifest, or `None` if not found
/// - `runbook_uris`: A vector of all runbook URIs referenced in the manifest
pub fn discover_workspace_files(workspace: &SharedWorkspaceState) -> DiscoveredFiles {
    let workspace_read = workspace.read();

    // Find manifest URI
    let manifest_uri = find_manifest_in_open_documents(&workspace_read)
        .or_else(|| search_manifest_from_open_documents(&workspace_read));

    // Extract runbooks from manifest
    let runbook_uris = manifest_uri
        .as_ref()
        .and_then(|uri| extract_runbook_uris(&workspace_read, uri))
        .unwrap_or_default();

    DiscoveredFiles { manifest_uri, runbook_uris }
}

/// Checks if manifest is already open in workspace
fn find_manifest_in_open_documents(
    workspace: &crate::cli::lsp::workspace::WorkspaceState,
) -> Option<Url> {
    workspace
        .documents()
        .iter()
        .find(|(uri, _)| is_manifest_file(uri))
        .map(|(uri, _)| uri.clone())
}

/// Searches for manifest by walking up from any open document
fn search_manifest_from_open_documents(
    workspace: &crate::cli::lsp::workspace::WorkspaceState,
) -> Option<Url> {
    workspace
        .documents()
        .iter()
        .find_map(|(uri, _)| search_for_manifest_from_path(uri))
}

/// Checks if a URI points to a manifest file based on filename
fn is_manifest_file(uri: &Url) -> bool {
    uri.path().ends_with("txtx.yml") || uri.path().ends_with("txtx.yaml")
}

/// Searches for manifest file by walking up directory tree from the given URI
fn search_for_manifest_from_path(uri: &Url) -> Option<Url> {
    let path = uri.to_file_path().ok()?;
    let root = file_scanner::find_txtx_yml_root(&path)?;

    // Try both txtx.yml and txtx.yaml
    ["txtx.yml", "txtx.yaml"]
        .iter()
        .find_map(|name| {
            let manifest_path = root.join(name);
            manifest_path
                .exists()
                .then(|| Url::from_file_path(&manifest_path).ok())?
        })
}

/// Extracts runbook URIs from the manifest
fn extract_runbook_uris(
    workspace: &crate::cli::lsp::workspace::WorkspaceState,
    manifest_uri: &Url,
) -> Option<Vec<Url>> {
    let manifest = workspace.get_manifest(manifest_uri)?;
    let uris = manifest
        .runbooks
        .iter()
        .filter_map(|runbook_ref| runbook_ref.absolute_uri.clone())
        .collect();
    Some(uris)
}

/// Finds all occurrences of an input name in YAML manifest content.
///
/// This function matches input keys in the manifest's YAML structure.
/// It matches keys that appear directly under environment definitions.
///
/// # Example YAML Structure
///
/// ```yaml
/// environments:
///   global:
///     chain_id: 11155111  # This would match "chain_id"
///     confirmations: 12   # This would match "confirmations"
///   sepolia:
///     chain_id: 11155111  # This would also match "chain_id"
/// ```
///
/// # Arguments
///
/// * `content` - The YAML content to search
/// * `input_name` - The input name to find (e.g., "confirmations")
///
/// # Returns
///
/// A vector of ranges where the input name appears as a key in YAML.
/// The ranges cover only the key name itself, not the colon or value.
pub fn find_input_in_yaml(content: &str, input_name: &str) -> Vec<Range> {
    let mut ranges = Vec::new();

    // Parse YAML to get the exact structure
    let Ok(yaml_value) = serde_yml::from_str::<serde_yml::Value>(content) else {
        return ranges;
    };

    let Some(yaml_mapping) = yaml_value.as_mapping() else {
        return ranges;
    };

    let Some(envs_section) = yaml_mapping.get(&serde_yml::Value::String("environments".to_string())) else {
        return ranges;
    };

    let Some(envs_mapping) = envs_section.as_mapping() else {
        return ranges;
    };

    // Find which environments contain this input
    let matching_envs: Vec<String> = envs_mapping
        .iter()
        .filter_map(|(env_key, env_value)| {
            let env_name = env_key.as_str()?;
            let env_map = env_value.as_mapping()?;
            if env_map.contains_key(&serde_yml::Value::String(input_name.to_string())) {
                Some(env_name.to_string())
            } else {
                None
            }
        })
        .collect();

    if matching_envs.is_empty() {
        return ranges;
    }

    // Now find the line positions, but only within environments section
    let lines: Vec<&str> = content.lines().collect();
    let pattern = format!(r"^\s*({}):\s*", regex::escape(input_name));
    let re = Regex::new(&pattern).expect("valid regex pattern");

    // Track whether we're inside the environments section
    let mut in_environments = false;
    let mut in_target_env = false;
    let mut current_indent = 0;
    let mut env_indent = 0;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check if we're entering the environments section
        if trimmed.starts_with("environments:") {
            in_environments = true;
            current_indent = line.len() - line.trim_start().len();
            continue;
        }

        // If we're in environments section
        if in_environments {
            let line_indent = line.len() - line.trim_start().len();

            // If we're back to the same or less indentation as "environments:", we've left the section
            if !trimmed.is_empty() && line_indent <= current_indent {
                in_environments = false;
                in_target_env = false;
                continue;
            }

            // Check if this line is an environment name (e.g., "global:")
            if trimmed.ends_with(':') && !trimmed.contains(' ') {
                let env_name = trimmed.trim_end_matches(':');
                in_target_env = matching_envs.contains(&env_name.to_string());
                env_indent = line_indent;
                continue;
            }

            // If we're in a target environment, check for the input key
            if in_target_env {
                // Make sure we're still inside the environment (more indented than env name)
                if !trimmed.is_empty() && line_indent <= env_indent {
                    in_target_env = false;
                    continue;
                }

                if let Some(cap) = re.captures(line) {
                    if let Some(name_match) = cap.get(1) {
                        ranges.push(Range {
                            start: Position {
                                line: line_idx as u32,
                                character: name_match.start() as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: name_match.end() as u32,
                            },
                        });
                    }
                }
            }
        }
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_input_in_yaml() {
        let yaml = r#"
environments:
  global:
    confirmations: 12
    timeout: 30
  sepolia:
    confirmations: 6
"#;

        let ranges = find_input_in_yaml(yaml, "confirmations");
        assert_eq!(ranges.len(), 2, "Should find 2 occurrences of 'confirmations'");

        // Verify first occurrence (global)
        assert_eq!(ranges[0].start.line, 3);
        assert_eq!(ranges[0].start.character, 4); // "    confirmations"

        // Verify second occurrence (sepolia)
        assert_eq!(ranges[1].start.line, 6);
        assert_eq!(ranges[1].start.character, 4);
    }

    #[test]
    fn test_find_input_in_yaml_only_under_environments() {
        let yaml = r#"
some_other_section:
  confirmations: 999
environments:
  global:
    confirmations: 12
"#;

        let ranges = find_input_in_yaml(yaml, "confirmations");
        // Should only find the one under "environments:", not the one in "some_other_section"
        assert_eq!(ranges.len(), 1, "Should only find 'confirmations' under environments:");
        assert_eq!(ranges[0].start.line, 5);
    }
}
