use std::collections::{HashMap, VecDeque};
use crate::manifest::WorkspaceManifest;
use crate::runbook::RunbookSources;
use crate::runbook::location::SourceLocation;
use crate::validation::hcl_validator::validate_with_hcl_and_addons;
use crate::validation::types::ValidationResult;
use crate::kit::types::commands::CommandSpecification;

/// Represents a variable used in a runbook
#[derive(Debug, Clone)]
pub struct RunbookVariable {
    /// Variable name (e.g., "operator_eoa")
    pub name: String,
    /// Full path as referenced (e.g., "input.operator_eoa")
    pub full_path: String,
    /// Resolved value from environment/manifest
    pub resolved_value: Option<String>,
    /// Where this variable is defined
    pub source: VariableSource,
    /// All places where this variable is referenced
    pub references: Vec<VariableReference>,
}

/// Source of a variable's value
#[derive(Debug, Clone)]
pub enum VariableSource {
    /// Defined in an environment in the manifest
    Environment { name: String },
    /// Would come from command-line --input
    CommandLineInput,
    /// Not defined anywhere
    Undefined,
}

/// A reference to a variable in the runbook
#[derive(Debug, Clone)]
pub struct VariableReference {
    /// Location where the reference appears
    pub location: SourceLocation,
    /// Context of the reference
    pub context: ReferenceContext,
}

/// Context where a variable is referenced
#[derive(Debug, Clone)]
pub enum ReferenceContext {
    /// Referenced in a signer block
    Signer { signer_name: String },
    /// Referenced in an action block
    Action { action_name: String },
    /// Referenced in an addon block
    Addon { addon_name: String },
    /// Referenced in an output block
    Output { output_name: String },
    /// Other context
    Other,
}

/// Iterator over variables in a runbook
pub struct RunbookVariableIterator {
    /// All variables found in the runbook
    variables: VecDeque<RunbookVariable>,
}

impl RunbookVariableIterator {
    /// Create a new iterator from runbook sources and manifest
    pub fn new(
        runbook_sources: &RunbookSources,
        manifest: &WorkspaceManifest,
        environment: Option<&str>,
        addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Result<Self, String> {
        Self::new_with_cli_inputs(runbook_sources, manifest, environment, addon_specs, &[])
    }

    /// Create a new iterator with CLI input overrides
    pub fn new_with_cli_inputs(
        runbook_sources: &RunbookSources,
        manifest: &WorkspaceManifest,
        environment: Option<&str>,
        addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
        cli_inputs: &[(String, String)],
    ) -> Result<Self, String> {
        let mut variables = HashMap::new();

        // Combine runbook content for validation
        let mut combined_content = String::new();
        let mut file_boundaries = Vec::new();
        let mut current_line = 1;

        for (file_location, (_name, raw_content)) in runbook_sources.tree.iter() {
            let path = file_location.to_string();
            let content = raw_content.to_string();
            let start_line = current_line;
            combined_content.push_str(&content);
            if !combined_content.ends_with('\n') {
                combined_content.push('\n');
            }
            let lines = content.lines().count();
            let end_line = current_line + lines;
            file_boundaries.push((path, start_line, end_line));
            current_line = end_line;
        }

        // Run HCL validation to collect input references
        let mut validation_result = ValidationResult::default();
        let refs = validate_with_hcl_and_addons(
            &combined_content,
            &mut validation_result,
            "runbook",
            addon_specs,
        )?;

        // Process collected input references
        for input_ref in refs.inputs {
            let var_name = Self::extract_variable_name(&input_ref.name);

            // Find which file this reference is in
            let file = Self::find_file_for_line(&file_boundaries, input_ref.line)
                .unwrap_or_else(|| "unknown".to_string());

            // Create or update variable entry
            let entry = variables.entry(var_name.clone()).or_insert_with(|| {
                let (resolved_value, source) = Self::resolve_variable(
                    &var_name,
                    manifest,
                    environment,
                    cli_inputs,
                );

                RunbookVariable {
                    name: var_name.clone(),
                    full_path: input_ref.name.clone(),
                    resolved_value,
                    source,
                    references: Vec::new(),
                }
            });

            // Add reference
            entry.references.push(VariableReference {
                location: SourceLocation::new(file.clone(), input_ref.line, input_ref.column),
                context: Self::determine_context(&input_ref.name),
            });
        }

        // Also check for signer references that map to input variables
        Self::process_signer_references(&mut variables, &validation_result, &file_boundaries, manifest, environment, cli_inputs);

        Ok(Self {
            variables: variables.into_values().collect(),
        })
    }

    /// Extract the variable name from a full path (e.g., "input.foo" -> "foo")
    fn extract_variable_name(full_path: &str) -> String {
        if let Some((_prefix, name)) = full_path.split_once('.') {
            name.to_string()
        } else {
            full_path.to_string()
        }
    }

    /// Find which file a line number belongs to
    fn find_file_for_line(file_boundaries: &[(String, usize, usize)], line: usize) -> Option<String> {
        for (file, start, end) in file_boundaries {
            if line >= *start && line < *end {
                return Some(file.clone());
            }
        }
        None
    }

    /// Resolve a variable's value from CLI inputs, then manifest
    fn resolve_variable(
        name: &str,
        manifest: &WorkspaceManifest,
        environment: Option<&str>,
        cli_inputs: &[(String, String)],
    ) -> (Option<String>, VariableSource) {
        // CLI inputs take precedence
        for (key, value) in cli_inputs {
            if key == name {
                return (Some(value.clone()), VariableSource::CommandLineInput);
            }
        }

        // Try environment-specific next
        if let Some(env_name) = environment {
            if let Some(env_vars) = manifest.environments.get(env_name) {
                if let Some(value) = env_vars.get(name) {
                    return (Some(value.clone()), VariableSource::Environment {
                        name: env_name.to_string()
                    });
                }
            }
        }

        // Try global environment
        if let Some(global_vars) = manifest.environments.get("global") {
            if let Some(value) = global_vars.get(name) {
                return (Some(value.clone()), VariableSource::Environment {
                    name: "global".to_string()
                });
            }
        }

        // Not found
        (None, VariableSource::Undefined)
    }

    /// Determine the context of a variable reference
    fn determine_context(_full_path: &str) -> ReferenceContext {
        // This would need to be enhanced with actual context tracking from the HCL visitor
        // For now, return a simple classification
        ReferenceContext::Other
    }

    /// Process signer references to find additional input variables
    fn process_signer_references(
        variables: &mut HashMap<String, RunbookVariable>,
        validation_result: &ValidationResult,
        _file_boundaries: &[(String, usize, usize)],
        manifest: &WorkspaceManifest,
        environment: Option<&str>,
        cli_inputs: &[(String, String)],
    ) {
        // Look for "Reference to undefined signer" errors
        for error in &validation_result.errors {
            if error.message.starts_with("Reference to undefined signer") {
                // Extract signer name
                if let Some(signer_name) = error.message.split('\'').nth(1) {
                    // Map signer.foo to input.foo_eoa or similar
                    // This is a simplified mapping - real implementation would need
                    // to understand the actual signer-to-input mapping rules
                    let input_name = if signer_name == "operator" {
                        "operator_eoa".to_string()
                    } else {
                        format!("{}_address", signer_name)
                    };

                    // Add if not already present
                    if !variables.contains_key(&input_name) {
                        let (resolved_value, source) = Self::resolve_variable(
                            &input_name,
                            manifest,
                            environment,
                            cli_inputs,
                        );

                        let file = error.file.clone().unwrap_or_default();

                        variables.insert(input_name.clone(), RunbookVariable {
                            name: input_name.clone(),
                            full_path: format!("input.{}", input_name),
                            resolved_value,
                            source,
                            references: vec![VariableReference {
                                location: SourceLocation::new(
                                    file,
                                    error.line.unwrap_or(0),
                                    error.column.unwrap_or(0)
                                ),
                                context: ReferenceContext::Signer {
                                    signer_name: signer_name.to_string()
                                },
                            }],
                        });
                    }
                }
            }
        }
    }

    /// Filter to only undefined variables
    pub fn undefined_only(self) -> impl Iterator<Item = RunbookVariable> {
        self.variables.into_iter().filter(|v| v.resolved_value.is_none())
    }

    /// Filter to undefined variables OR those provided via CLI
    pub fn undefined_or_cli_provided(self) -> impl Iterator<Item = RunbookVariable> {
        self.variables.into_iter().filter(|v| {
            v.resolved_value.is_none() || matches!(v.source, VariableSource::CommandLineInput)
        })
    }

    /// Filter to only defined variables
    pub fn defined_only(self) -> impl Iterator<Item = RunbookVariable> {
        self.variables.into_iter().filter(|v| v.resolved_value.is_some())
    }
}

impl Iterator for RunbookVariableIterator {
    type Item = RunbookVariable;

    fn next(&mut self) -> Option<Self::Item> {
        self.variables.pop_front()
    }
}