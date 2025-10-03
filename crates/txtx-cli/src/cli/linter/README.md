# txtx Linter Module

## Overview

The txtx linter provides validation and formatting capabilities for txtx runbooks and manifests. It has been refactored to provide a simpler, more maintainable architecture.

## Architecture

### Module Structure

```
linter/
├── mod.rs         # Public API and exports
├── config.rs      # Configuration types
├── rules.rs       # Validation rules
├── validator.rs   # Validation engine
├── formatter.rs   # Output formatters
└── workspace.rs   # Workspace analysis
```

### Key Components

#### 1. Linter (`validator.rs`)

The main entry point for validation:

```rust
use txtx_cli::cli::linter::{Linter, LinterConfig, Format};

// Create configuration
let config = LinterConfig::new(
    Some(manifest_path),
    Some("my_runbook".to_string()),
    Some("production".to_string()),
    vec![("key".to_string(), "value".to_string())],
    Format::Json,
);

// Create linter and validate
let linter = Linter::new(&config)?;
let result = linter.lint_runbook("my_runbook")?;
```

#### 2. Validation Rules (`rules.rs`)

All validation rules implement the `ValidationRule` trait:

```rust
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, context: &ValidationContext) -> ValidationOutcome;
}
```

Available rules:
- `InputDefinedRule`: Checks that all input references are defined
- `NamingConventionRule`: Enforces naming conventions
- `CliOverrideRule`: Warns when CLI inputs override manifest values
- `SensitiveDataRule`: Detects potential sensitive data exposure

#### 3. Formatters (`formatter.rs`)

Output formatters for different use cases:

- `PlainFormatter`: Human-readable plain text
- `JsonFormatter`: Machine-readable JSON
- `GithubFormatter`: GitHub Actions annotations
- `CsvFormatter`: CSV export for analysis

## Adding New Rules

To add a new validation rule:

1. Create a new struct implementing `ValidationRule`:

```rust
pub struct MyCustomRule;

impl ValidationRule for MyCustomRule {
    fn name(&self) -> &'static str {
        "my-custom-rule"
    }

    fn check(&self, context: &ValidationContext) -> ValidationOutcome {
        // Access the input being validated
        let input = &context.input;

        // Perform validation logic
        if some_condition {
            ValidationOutcome::Error {
                message: "Validation failed".to_string(),
                context: Some("Additional context".to_string()),
                suggestion: Some("How to fix".to_string()),
                documentation_link: None,
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}
```

2. Add the rule to the linter in `validator.rs`:

```rust
impl Linter {
    pub fn new(config: &LinterConfig) -> Result<Self, String> {
        let rules: Vec<Box<dyn ValidationRule>> = vec![
            Box::new(rules::InputDefinedRule),
            Box::new(rules::MyCustomRule), // Add your rule here
            // ... other rules
        ];

        Ok(Self { rules, config: config.clone() })
    }
}
```

## API Usage

### Programmatic Usage

```rust
use txtx_cli::cli::linter::{lint_content, run_linter};

// Lint a string content
let result = lint_content(
    content,
    "path/to/file.txtx",
    Some(manifest_path),
    Some("production".to_string()),
);

// Run full linter
run_linter(
    Some(manifest_path),
    Some("my_runbook".to_string()),
    Some("production".to_string()),
    vec![],
    Format::Json,
)?;
```

### CLI Usage

```bash
# Lint all runbooks
txtx lint

# Lint specific runbook
txtx lint --runbook my_runbook

# Lint with specific environment
txtx lint --env production

# Output as JSON
txtx lint --format json

# Output as GitHub annotations
txtx lint --format github
```

## Configuration

The linter can be configured through `LinterConfig`:

```rust
pub struct LinterConfig {
    pub manifest_path: Option<PathBuf>,
    pub runbook: Option<String>,
    pub environment: Option<String>,
    pub cli_inputs: Vec<(String, String)>,
    pub format: Format,
}
```

## Performance Considerations

- The linter is stateless - a new instance is created for each validation
- Rules are executed sequentially for each input
- File I/O is minimized through caching in the workspace analyzer
- The linter is designed to be fast enough for real-time LSP validation

## Testing

Test utilities are available for writing rule tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::linter::test_utils::*;

    #[test]
    fn test_my_rule() {
        let context = create_test_context("input.some_value");
        let rule = MyCustomRule;
        let outcome = rule.check(&context);
        assert!(matches!(outcome, ValidationOutcome::Pass));
    }
}
```

## Migration from Old API

If you were using the old linter API:

**Before:**
```rust
use txtx_cli::cli::linter_impl::RunbookAnalyzer;

let analyzer = RunbookAnalyzer::new(config);
let result = analyzer.analyze()?;
```

**After:**
```rust
use txtx_cli::cli::linter::{Linter, LinterConfig};

let linter = Linter::new(&config)?;
let result = linter.lint_all()?;
```

Key changes:
- `RunbookAnalyzer` → `Linter`
- `analyze()` → `lint_all()` or `lint_runbook()`
- Simpler configuration structure
- Direct rule access for testing