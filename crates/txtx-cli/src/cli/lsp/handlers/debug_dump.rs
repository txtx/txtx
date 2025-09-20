//! Debug dump handlers for LSP hover
//!
//! Provides debug information dumps for txtx state and variables

use super::environment_resolver::EnvironmentResolver;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use crate::cli::lsp::utils::environment;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Url};

pub struct DebugDumpHandler {
    workspace: SharedWorkspaceState,
}

impl DebugDumpHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace }
    }



    /// Dump the current txtx state for debugging
    pub fn dump_state(&self, uri: &Url) -> Option<Hover> {
        let workspace = self.workspace.read();
        
        // Get the current environment
        let current_env = workspace.get_current_environment()
            .or_else(|| environment::extract_environment_from_uri(uri))
            .unwrap_or_else(|| "global".to_string());
        
        let mut debug_text = String::from("# 🔍 txtx State Dump\n\n");
        
        // Add current file info
        debug_text.push_str(&format!("**Current file**: `{}`\n", uri.path()));
        debug_text.push_str(&format!("**Selected environment**: `{}`\n", current_env));
        
        // Add environment detection info
        if let Some(file_env) = environment::extract_environment_from_uri(uri) {
            if file_env != current_env {
                debug_text.push_str(&format!("**File-based environment**: `{}` (overridden by selector)\n", file_env));
            }
        }
        debug_text.push_str("\n");
        
        // Get manifest info
        if let Some(manifest) = workspace.get_manifest_for_document(uri) {
            let resolver = EnvironmentResolver::new(&manifest, current_env.clone());
            
            debug_text.push_str("## Manifest Information\n\n");
            debug_text.push_str(&format!("**Manifest URI**: `{}`\n\n", manifest.uri));
            
            // List all environments
            debug_text.push_str("## Environments\n\n");
            let env_names = resolver.get_all_environments();
            
            for env_name in &env_names {
                if let Some(env_vars) = manifest.environments.get(env_name) {
                    debug_text.push_str(&format!("### {} ({} variables)\n", env_name, env_vars.len()));
                    
                    // Sort variables by key
                    let mut vars: Vec<_> = env_vars.iter().collect();
                    vars.sort_by_key(|(k, _)| k.as_str());
                    
                    if vars.is_empty() {
                        debug_text.push_str("*(no variables)*\n");
                    } else {
                        // Show first few variables as a sample
                        debug_text.push_str("```yaml\n");
                        for (idx, (key, value)) in vars.iter().enumerate() {
                            if idx < 10 {
                                // Truncate long values for display
                                let display_value = truncate_value(value, 50);
                                debug_text.push_str(&format!("{}: \"{}\"\n", key, display_value));
                            } else if idx == 10 {
                                debug_text.push_str(&format!("# ... and {} more variables\n", vars.len() - 10));
                                break;
                            }
                        }
                        debug_text.push_str("```\n");
                    }
                    debug_text.push('\n');
                }
            }
            
            // Show effective inputs for current environment
            debug_text.push_str(&format!("## Effective Inputs for '{}'\n\n", current_env));
            debug_text.push_str("*Resolution order: CLI inputs > environment-specific > global*\n\n");
            
            let effective_inputs = resolver.get_effective_inputs();
            
            // Sort and display effective inputs
            let mut effective_vars: Vec<_> = effective_inputs.iter().collect();
            effective_vars.sort_by_key(|(k, _)| k.as_str());
            
            debug_text.push_str(&format!("**Total resolved inputs**: {}\n\n", effective_vars.len()));
            
            if effective_vars.is_empty() {
                debug_text.push_str("*(no inputs available)*\n");
            } else {
                debug_text.push_str("```yaml\n");
                for (idx, (key, (value, source))) in effective_vars.iter().enumerate() {
                    if idx < 20 {
                        // Truncate long values for display
                        let display_value = truncate_value(value, 50);
                        
                        if source == &current_env {
                            debug_text.push_str(&format!("{}: \"{}\"  # from {}\n", key, display_value, source));
                        } else {
                            debug_text.push_str(&format!("{}: \"{}\"  # inherited from {}\n", key, display_value, source));
                        }
                    } else if idx == 20 {
                        debug_text.push_str(&format!("# ... and {} more inputs\n", effective_vars.len() - 20));
                        break;
                    }
                }
                debug_text.push_str("```\n");
            }
            
            // Show summary statistics
            debug_text.push_str("\n## Summary\n\n");
            let global_count = manifest.environments.get("global").map_or(0, |e| e.len());
            let env_count = if current_env != "global" {
                manifest.environments.get(&current_env).map_or(0, |e| e.len())
            } else {
                0
            };
            
            debug_text.push_str(&format!("- **Global inputs**: {}\n", global_count));
            if current_env != "global" {
                debug_text.push_str(&format!("- **{} inputs**: {} (overrides)\n", current_env, env_count));
            }
            debug_text.push_str(&format!("- **Total effective inputs**: {}\n", effective_vars.len()));
            
            // List all available environments
            debug_text.push_str(&format!("\n**Available environments**: {}\n", 
                env_names.join(", ")));
            
        } else {
            debug_text.push_str("## ⚠️ No manifest found\n\n");
            debug_text.push_str("Could not find a `txtx.yml` file in the workspace.\n");
        }
        
        // Add workspace info
        debug_text.push_str("\n## Workspace Information\n\n");
        debug_text.push_str(&format!("**VS Code environment selector**: {}\n", 
            workspace.get_current_environment().unwrap_or_else(|| "not set".to_string())));
        debug_text.push_str(&format!("**Documents loaded**: {}\n", 
            workspace.documents().len()));
        
        // Add debugging tips
        debug_text.push_str("\n---\n");
        debug_text.push_str("💡 **Tip**: Use `input.dump_txtx_state` in any `.tx` file to see this debug info.\n");
        debug_text.push_str("💡 **Tip**: Use the VS Code environment selector to switch environments.\n");
        
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: debug_text,
            }),
            range: None,
        })
    }

    /// Dump detailed information about a specific variable across all environments
    pub fn dump_variable(&self, uri: &Url, variable_name: &str) -> Option<Hover> {
        let workspace = self.workspace.read();

        // Get the current environment
        let current_env = workspace.get_current_environment()
            .or_else(|| environment::extract_environment_from_uri(uri))
            .unwrap_or_else(|| "global".to_string());

        let mut debug_text = format!("# 🔍 Variable Details: `{}`\n\n", variable_name);

        // Add current environment info
        debug_text.push_str(&format!("**Current environment**: `{}`\n\n", current_env));

        // Get manifest info
        if let Some(manifest) = workspace.get_manifest_for_document(uri) {
            let resolver = EnvironmentResolver::new(&manifest, current_env.clone());
            
            // Get all values for this variable
            let env_values = resolver.get_all_values(variable_name);

            // Show definition in each environment
            debug_text.push_str("## Variable Definitions by Environment\n\n");

            let global_value = manifest.environments.get("global")
                .and_then(|vars| vars.get(variable_name))
                .cloned();

            for (env_name, value) in &env_values {
                debug_text.push_str(&format!("### `{}`\n", env_name));

                // Show the actual value
                let display_value = truncate_value(&value, 100);
                debug_text.push_str(&format!("**Value**: `{}`\n", display_value));

                // Indicate if it's an override
                if env_name != "global" && global_value.is_some() && global_value.as_ref() != Some(value) {
                    debug_text.push_str("*⚡ Overrides global value*\n");
                }

                debug_text.push_str("\n");
            }

            // Show environments that don't define this variable but inherit it
            debug_text.push_str("## Environment Resolution\n\n");

            let env_names = resolver.get_all_environments();
            for env_name in &env_names {
                debug_text.push_str(&format!("### `{}`", env_name));

                // Mark current environment
                if env_name == &current_env {
                    debug_text.push_str(" *(current)*");
                }
                debug_text.push_str("\n");

                // Check if defined locally
                let local_value = manifest.environments.get(env_name)
                    .and_then(|vars| vars.get(variable_name));

                if let Some(val) = local_value {
                    let display_value = truncate_value(val, 100);
                    debug_text.push_str(&format!("- **Defined locally**: `{}`\n", display_value));
                } else if env_name != "global" {
                    // Check if inherited from global
                    if let Some(ref global_val) = global_value {
                        let display_value = truncate_value(global_val, 100);
                        debug_text.push_str(&format!("- **Inherited from global**: `{}`\n", display_value));
                    } else {
                        debug_text.push_str("- **Not defined** (variable not available)\n");
                    }
                } else {
                    debug_text.push_str("- **Not defined** (variable not available)\n");
                }

                // Show the resolved value
                if let Some((resolved, _)) = EnvironmentResolver::new(&manifest, env_name.clone()).resolve_value(variable_name) {
                    let display_value = truncate_value(&resolved, 100);
                    debug_text.push_str(&format!("- **Resolved value**: `{}`\n", display_value));
                }

                debug_text.push_str("\n");
            }

            // Summary
            debug_text.push_str("## Summary\n\n");

            let defined_count = env_values.len();
            let total_envs = env_names.len();

            debug_text.push_str(&format!("- **Variable name**: `{}`\n", variable_name));
            debug_text.push_str(&format!("- **Defined in**: {} of {} environments\n", defined_count, total_envs));

            if let Some(ref global_val) = global_value {
                let display_value = truncate_value(global_val, 50);
                debug_text.push_str(&format!("- **Global value**: `{}`\n", display_value));

                // Count overrides
                let override_count = resolver.count_overrides(variable_name);

                if override_count > 0 {
                    debug_text.push_str(&format!("- **Overridden in**: {} environment(s)\n", override_count));
                }
            } else {
                debug_text.push_str("- **Global value**: *not defined*\n");
            }

            // Check current environment resolution
            if let Some((resolved, source)) = resolver.resolve_value(variable_name) {
                let display_value = truncate_value(&resolved, 50);
                debug_text.push_str(&format!("\n**Resolved in current environment (`{}`)**: `{}`\n",
                    current_env, display_value));
            } else {
                debug_text.push_str(&format!("\n⚠️ **Not available in current environment (`{}`)**\n", current_env));
            }

        } else {
            debug_text.push_str("## ⚠️ No manifest found\n\n");
            debug_text.push_str("Could not find a `txtx.yml` file in the workspace.\n");
        }

        // Add tip
        debug_text.push_str("\n---\n");
        debug_text.push_str(&format!("💡 **Tip**: Use `input.dump_txtx_var_<name>` to see details for any variable.\n"));

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: debug_text,
            }),
            range: None,
        })
    }
}

/// Helper function to truncate long values for display
fn truncate_value(value: &str, max_len: usize) -> String {
    if value.len() > max_len {
        format!("{}...", &value[..max_len.saturating_sub(3)])
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_truncate_value() {
        assert_eq!(truncate_value("short", 10), "short");
        assert_eq!(truncate_value("this is a very long value", 10), "this is...");
        assert_eq!(truncate_value("exact", 5), "exact");
        assert_eq!(truncate_value("toolong", 5), "to...");
    }
}