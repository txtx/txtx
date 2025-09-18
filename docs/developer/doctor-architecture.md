# txtx Doctor Command Architecture

## Overview

The doctor command is implemented as a modular, extensible validation system that uses trait-based design patterns. It leverages the HCL parser and addon system to provide comprehensive validation of txtx runbooks.

## Module Structure

```console
crates/txtx-cli/src/cli/doctor/
├── mod.rs              # Main orchestrator (195 lines)
├── config.rs           # Configuration management
├── workspace.rs        # Workspace and runbook discovery
├── analyzer/
│   ├── mod.rs         # Core analyzer
│   ├── rules.rs       # Validation rules (trait-based)
│   ├── validator.rs   # Rule execution engine
│   └── inputs.rs      # Input validation helpers
└── formatter/
    ├── mod.rs         # Formatter trait
    ├── terminal.rs    # Pretty terminal output
    ├── json.rs        # JSON output
    └── quickfix.rs    # Editor integration format
```

## Key Components

### 1. Main Orchestrator ([`mod.rs`](crates/txtx-cli/src/cli/doctor/mod.rs))

The main entry point that coordinates the validation flow:

```rust
pub fn run_doctor(
    manifest_path: Option<String>,
    runbook_name: Option<String>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
    format: DoctorOutputFormat,
) -> Result<(), String> {
    let config = DoctorConfig::new(manifest_path, runbook_name, environment, cli_inputs, format)
        .resolve_format();

    match config.runbook_name {
        Some(ref name) => run_specific_runbook(&config, name),
        None => run_all_runbooks(&config),
    }
}
```

### 2. Validation Rule System ([`analyzer/rules.rs`](crates/txtx-cli/src/cli/doctor/analyzer/rules.rs))

The core trait that all validation rules implement:

```rust
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, context: &ValidationContext) -> ValidationOutcome;
    fn description(&self) -> &'static str {
        "No description provided"
    }
}

pub struct ValidationContext<'a> {
    pub input_name: &'a str,
    pub full_name: &'a str,
    pub manifest: &'a WorkspaceManifest,
    pub environment: Option<&'a str>,
    pub effective_inputs: &'a HashMap<String, String>,
    pub cli_inputs: &'a [(String, String)],
    pub content: &'a str,
    pub file_path: &'a str,
}
```

### 3. HCL-Based Validation ([`txtx_core::validation::hcl_validator`](crates/txtx-core/src/validation/hcl_validator.rs))

Uses the `hcl-edit` visitor pattern for comprehensive AST validation:

```rust
pub struct HclValidationVisitor<'a> {
    result: &'a mut ValidationResult,
    file_path: String,
    source: &'a str,
    
    // Collection phase data
    action_types: HashMap<String, String>,
    action_specs: HashMap<String, CommandSpecification>,
    addon_specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    defined_variables: HashSet<String>,
    defined_signers: HashMap<String, String>,
    defined_outputs: HashSet<String>,
    flow_inputs: HashMap<String, Vec<String>>,
    
    // Context tracking
    current_block: Option<BlockContext>,
    is_validation_phase: bool,
}
```

### 4. Output Formatters

#### Formatter Trait ([`formatter/mod.rs`](crates/txtx-cli/src/cli/doctor/formatter/mod.rs))

```rust
pub trait OutputFormatter {
    fn format_results(&self, result: &ValidationResult) -> String;
}
```

#### Terminal Formatter ([`formatter/terminal.rs`](crates/txtx-cli/src/cli/doctor/formatter/terminal.rs))

Provides colored, human-readable output with clickable error locations:

```rust
pub fn display(result: &ValidationResult) {
    let total_issues = result.errors.len() + result.warnings.len();
    
    if total_issues == 0 {
        println!("{} No issues found!", Blue.paint("✓"));
        return;
    }
    
    println!("{}", Red.bold().paint(format!("Found {} issue(s):", total_issues)));
    display_errors(result);
    display_warnings(result);
    display_suggestions(result);
}
```

## Validation Flow

### 1. Configuration Phase

```text
DoctorConfig::new() → resolve_format() → determine output format
```

### 2. Discovery Phase

```text
WorkspaceAnalyzer::new() → discover_runbooks() → find all .tx files
```

### 3. Collection Phase

The HCL visitor collects all definitions in a first pass:

- Actions and their types
- Variables and signers
- Outputs
- Flow definitions

For multi-file runbooks, the analyzer combines content from all files:

```rust
// In analyze_runbook_with_manifest()
if runbook_sources.tree.len() > 1 {
    let mut combined_content = String::new();
    let mut file_boundaries = Vec::new();
    let mut current_line = 1usize;
    
    for (file_location, (_name, raw_content)) in &runbook_sources.tree {
        let start_line = current_line;
        let content = raw_content.to_string();
        combined_content.push_str(&content);
        combined_content.push('\n');
        let line_count = content.lines().count();
        current_line += line_count + 1;
        file_boundaries.push((file_location.to_string(), start_line, current_line));
    }
}
```

### 4. Validation Phase

The visitor validates references in a second pass:

- Input references exist
- Action outputs are valid
- Signer references are defined
- No circular dependencies

For multi-file runbooks, errors are mapped back to their original files using the `file_boundaries` tracking.

### 5. Formatting Phase

```console
ValidationResult → OutputFormatter → Terminal/JSON/Quickfix
```

## Adding New Validation Rules

### Step 1: Create the Rule

```rust
// In analyzer/rules.rs
pub struct MyCustomRule;

impl ValidationRule for MyCustomRule {
    fn name(&self) -> &'static str {
        "custom_rule"
    }
    
    fn description(&self) -> &'static str {
        "Validates custom business logic"
    }
    
    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        // Implement validation logic
        if some_condition {
            ValidationOutcome::Pass
        } else {
            ValidationOutcome::Error {
                message: "Custom validation failed".to_string(),
                context: Some("Add context here".to_string()),
                suggestion: Some(ValidationSuggestion {
                    message: "How to fix it".to_string(),
                    example: Some("Example code".to_string()),
                }),
                documentation_link: Some("https://docs.txtx.sh/...".to_string()),
            }
        }
    }
}
```

### Step 2: Register the Rule

```rust
// In analyzer/validator.rs
pub fn get_validation_rules() -> Vec<Box<dyn ValidationRule>> {
    vec![
        Box::new(InputDefinedRule),
        Box::new(InputNamingConventionRule),
        Box::new(MyCustomRule), // Add here
    ]
}
```

## Integration Points

### 1. LSP Integration

The LSP uses the same validation infrastructure:

```rust
// In lsp/diagnostics.rs
pub fn validate_runbook(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
    let addons = addon_registry::get_all_addons();
    let addon_specs = addon_registry::extract_addon_specifications(&addons);
    
    match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
        content,
        &mut validation_result,
        file_path,
        addon_specs,
    ) {
        // Convert to LSP diagnostics
    }
}
```

### 2. Addon System Integration

The doctor command loads all addon specifications:

```rust
// In common/addon_registry.rs
pub fn get_all_addons() -> Vec<Box<dyn ProvideAddons>> {
    vec![
        Box::new(txtx_addon_network_bitcoin::BitcoinAddon::new()),
        Box::new(txtx_addon_network_evm::EvmAddon::new()),
        Box::new(txtx_addon_network_svm::SvmAddon::new()),
        // ... other addons
    ]
}
```

### 3. Manifest Integration

Doctor validates against the workspace manifest:

```rust
// In workspace.rs
pub fn analyze_runbook(&self, runbook: &RunbookLocation, config: &DoctorConfig) 
    -> Result<AnalysisResult, String> {
    let manifest = self.load_manifest()?;
    let effective_inputs = self.resolve_inputs(&manifest, config)?;
    // ... validation
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_input_validation_rule() {
        let rule = InputDefinedRule;
        let ctx = create_test_context();
        
        match rule.check(&ctx) {
            ValidationOutcome::Pass => assert!(true),
            _ => panic!("Expected validation to pass"),
        }
    }
}
```

### Integration Tests

Located in `tests/fixtures/doctor/`:

- `missing_inputs.tx` - Tests input validation
- `invalid_outputs.tx` - Tests output validation
- `circular_deps.tx` - Tests dependency checking

### Running Tests

```bash
# Run doctor-specific tests
cargo test --package txtx-cli doctor

# Run with output
cargo test --package txtx-cli doctor -- --nocapture
```

## Performance Considerations

### 1. Lazy File Reading

Files are only read when needed, not all at once.

### 2. Parallel Validation

When validating multiple runbooks, they can be processed in parallel:

```rust
runbooks.par_iter()
    .map(|runbook| analyzer.analyze(runbook))
    .collect()
```

### 3. Caching

The addon specifications are loaded once and reused across all validations.

## Error Handling

The doctor command uses string-based errors for simplicity, but could be migrated to use `error-stack`:

```rust
// Current
pub fn run_doctor(...) -> Result<(), String>

// Future with error-stack
pub fn run_doctor(...) -> Result<(), Report<DoctorError>>
```

## Future Enhancements

### 1. Custom Rule Plugins

Allow users to define custom validation rules in their projects.

### 2. Auto-fix Support

Some issues could be automatically fixed:

```rust
pub trait AutoFixableRule: ValidationRule {
    fn can_fix(&self) -> bool;
    fn apply_fix(&self, context: &mut FixContext) -> Result<(), String>;
}
```

### 3. Incremental Validation

Only re-validate changed files for better performance in large projects.

### 4. Rule Configuration

Allow rules to be configured in `txtx.yml`:

```yaml
doctor:
  rules:
    naming_convention:
      pattern: "^[a-z_]+$"
    custom_rule:
      enabled: false
```

## Multi-File Runbook Support

The doctor command fully supports multi-file runbooks (directories containing multiple `.tx` files):

1. **Content Combination**: All files are combined into a single content string for validation
2. **Boundary Tracking**: Line number boundaries are tracked for each file
3. **Error Mapping**: Validation errors are mapped back to their original file and line number

```rust
// Error mapping for multi-file runbooks
for error in result.errors {
    if let Some(line) = error.line {
        for (file_path, start_line, end_line) in &file_boundaries {
            if line >= *start_line && line < *end_line {
                let mut mapped_error = error.clone();
                mapped_error.file = file_path.clone();
                mapped_error.line = Some(line - start_line);
                final_result.errors.push(mapped_error);
                break;
            }
        }
    }
}
```

## Code References

- Main entry: [`crates/txtx-cli/src/cli/doctor/mod.rs:20`](crates/txtx-cli/src/cli/doctor/mod.rs:20)
- Validation rules: [`crates/txtx-cli/src/cli/doctor/analyzer/rules.rs`](crates/txtx-cli/src/cli/doctor/analyzer/rules.rs)
- HCL validator: [`crates/txtx-core/src/validation/hcl_validator.rs`](crates/txtx-core/src/validation/hcl_validator.rs)
- Output formatters: [`crates/txtx-cli/src/cli/doctor/formatter/`](crates/txtx-cli/src/cli/doctor/formatter/)
- Test fixtures: [`tests/fixtures/doctor/`](tests/fixtures/doctor/)
- Pattern detection example: [`crates/txtx-cli/examples/test_doctor_pattern.rs`](/crates/txtx-cli/examples/test_doctor_pattern.rs) - Demonstrates how doctor detects common EVM send_eth output access errors

## See Also

- [DOCTOR_COMMAND.md](DOCTOR_COMMAND.md) - User documentation
- [CODING_PRINCIPLES.md](CODING_PRINCIPLES.md) - Architectural patterns used
- [ARCHITECTURAL_REFACTORING.md](ARCHITECTURAL_REFACTORING.md) - Refactoring details
