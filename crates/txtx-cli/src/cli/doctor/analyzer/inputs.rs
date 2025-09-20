use std::path::Path;
use txtx_core::{
    manifest::WorkspaceManifest,
    validation::{
        get_doctor_rules, get_strict_doctor_rules, LocatedInputRef, ManifestValidationConfig,
        ValidationContext, ValidationContextExt, ValidationResult,
    },
};

/// Validate input references against manifest environment
///
/// This function now uses ValidationContext for cleaner parameter passing
pub fn validate_inputs_against_manifest(
    input_refs: &[LocatedInputRef],
    content: &str,
    manifest: &WorkspaceManifest,
    environment: Option<&String>,
    result: &mut ValidationResult,
    file_path: &Path,
    cli_inputs: &[(String, String)],
) {
    // Create validation context with all necessary data
    let mut context = ValidationContext::new(content.to_string(), file_path.to_string_lossy())
        .with_manifest(manifest.clone())
        .with_cli_inputs(cli_inputs.to_vec());

    // Set environment if provided
    if let Some(env) = environment {
        context = context.with_environment(env.clone());
    }

    // Add the input refs collected from HCL validation
    for input_ref in input_refs {
        context.add_input_ref(input_ref.clone());
    }

    // Create configuration with doctor rules based on environment
    let config = if environment == Some(&"production".to_string())
        || environment == Some(&"prod".to_string())
    {
        let mut cfg = ManifestValidationConfig::strict();
        // Add doctor-specific rules for production
        cfg.custom_rules.extend(get_strict_doctor_rules());
        cfg
    } else {
        let mut cfg = ManifestValidationConfig::default();
        // Add standard doctor rules
        cfg.custom_rules.extend(get_doctor_rules());
        cfg
    };

    // Run manifest validation with the context
    context.validate_manifest(config, result);
}
