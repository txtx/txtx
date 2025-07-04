# Error-Stack Integration Spike for txtx

## Overview

This document explores integrating the `error-stack` library into the txtx project to enhance error reporting with structured, context-rich error handling.

## Why error-stack?

The error-stack library addresses several weaknesses identified in our initial audit:

1. **Rich Context**: Automatically captures and preserves error context as it propagates
2. **Structured Reporting**: Type-safe error boundaries with explicit context changes
3. **Attachments**: Can attach arbitrary data (documentation, examples, spans) to errors
4. **Backtraces**: Automatic backtrace capture for debugging
5. **Error Chaining**: Built-in support for error chains with parent relationships

## Integration Strategy

### Phase 1: Define Core Error Types

Replace the current `Diagnostic` with error-stack based types:

```rust
use error_stack::{Report, ResultExt, Context};

#[derive(Debug)]
pub enum TxtxError {
    Parsing,
    Validation,
    Execution,
    Network,
    TypeMismatch,
    MissingInput,
}

impl fmt::Display for TxtxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxtxError::Parsing => write!(f, "Failed to parse runbook"),
            TxtxError::Validation => write!(f, "Validation failed"),
            TxtxError::Execution => write!(f, "Execution failed"),
            TxtxError::Network => write!(f, "Network operation failed"),
            TxtxError::TypeMismatch => write!(f, "Type mismatch"),
            TxtxError::MissingInput => write!(f, "Missing required input"),
        }
    }
}

impl Context for TxtxError {}
```

### Phase 2: Create Domain-Specific Error Types

For each addon and major component:

```rust
// For EVM addon
#[derive(Debug)]
pub enum EvmError {
    InvalidAddress,
    TransactionFailed,
    ContractDeploymentFailed,
    InsufficientFunds,
}

impl fmt::Display for EvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvmError::InvalidAddress => write!(f, "Invalid Ethereum address"),
            EvmError::TransactionFailed => write!(f, "Transaction failed"),
            EvmError::ContractDeploymentFailed => write!(f, "Contract deployment failed"),
            EvmError::InsufficientFunds => write!(f, "Insufficient funds"),
        }
    }
}

impl Context for EvmError {}
```

### Phase 3: Attachment Types for Rich Context

Define structured attachments to replace current diagnostic fields:

```rust
#[derive(Debug)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug)]
pub struct ErrorDocumentation {
    pub help: String,
    pub example: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug)]
pub struct ErrorSpan {
    pub start: usize,
    pub end: usize,
    pub source_text: String,
}

#[derive(Debug)]
pub struct ActionContext {
    pub action_name: String,
    pub namespace: String,
    pub construct_id: String,
}
```

### Phase 4: Error Creation Patterns

Replace `diagnosed_error!` with error-stack patterns:

```rust
// Current pattern:
diagnosed_error!("unable to parse contract_id ({})", id)

// New pattern with error-stack:
fn parse_contract_id(id: &str) -> Result<ContractId, Report<EvmError>> {
    ContractId::from_str(id)
        .change_context(EvmError::InvalidAddress)
        .attach_printable(format!("contract_id: {}", id))
        .attach(ErrorDocumentation {
            help: "Contract IDs must be valid Ethereum addresses (0x followed by 40 hex characters)".into(),
            example: Some("0x1234567890abcdef1234567890abcdef12345678".into()),
            link: Some("https://docs.txtx.io/evm/addresses".into()),
        })
}
```

### Phase 5: Error Propagation

Update error propagation to preserve context:

```rust
// In action execution
pub async fn execute_action(
    &self,
    inputs: &CommandInputs,
) -> Result<CommandExecutionResult, Report<TxtxError>> {
    let address = self.parse_address(&inputs.address)
        .change_context(TxtxError::Execution)
        .attach_printable("Failed to execute deploy_contract action")
        .attach(ActionContext {
            action_name: self.name.clone(),
            namespace: "evm".into(),
            construct_id: self.id.clone(),
        })?;
    
    let result = self.deploy_contract(address)
        .await
        .attach_printable("Network call failed")?;
    
    Ok(result)
}
```

### Phase 6: Error Display

Implement rich error display using error-stack's formatting:

```rust
pub fn display_error(error: &Report<TxtxError>) {
    // error-stack provides detailed formatting out of the box
    eprintln!("{:?}", error);
    
    // Custom formatting for specific attachments
    if let Some(location) = error.request_ref::<ErrorLocation>() {
        eprintln!("  at {}:{}:{}", location.file, location.line, location.column);
    }
    
    if let Some(docs) = error.request_ref::<ErrorDocumentation>() {
        eprintln!("\nHelp: {}", docs.help);
        if let Some(example) = &docs.example {
            eprintln!("Example:\n{}", example);
        }
    }
}
```

## Migration Plan

### Step 1: Add Dependency
```toml
[dependencies]
error-stack = { version = "0.5", default-features = false }
```

### Step 2: Create Compatibility Layer

During migration, create a compatibility layer:

```rust
impl From<Diagnostic> for Report<TxtxError> {
    fn from(diag: Diagnostic) -> Self {
        let base_error = match diag.level {
            DiagnosticLevel::Error => TxtxError::Execution,
            _ => TxtxError::Execution, // Map appropriately
        };
        
        let mut report = Report::new(base_error)
            .attach_printable(diag.message);
        
        if let Some(location) = diag.location {
            report = report.attach(ErrorLocation {
                file: location.to_string(),
                line: diag.span.as_ref().map(|s| s.line_start).unwrap_or(0),
                column: diag.span.as_ref().map(|s| s.column_start).unwrap_or(0),
            });
        }
        
        if let Some(doc) = diag.documentation {
            report = report.attach(ErrorDocumentation {
                help: doc,
                example: diag.example,
                link: None,
            });
        }
        
        report
    }
}
```

### Step 3: Gradual Migration

1. Start with core error types in `txtx-core`
2. Migrate one addon at a time
3. Update CLI error display
4. Remove old `Diagnostic` type once migration is complete

## Benefits

1. **Automatic Context**: Error context is automatically preserved and enhanced as errors propagate
2. **Type Safety**: Strongly typed error boundaries prevent context loss
3. **Rich Attachments**: Can attach any data type for debugging
4. **Better Stack Traces**: Automatic backtrace capture
5. **Standardized Display**: Consistent error formatting out of the box

## Considerations

1. **Learning Curve**: Team needs to understand error-stack patterns
2. **Refactoring Effort**: Significant changes to error handling code
3. **Dependency**: Adds external dependency (though well-maintained)
4. **API Changes**: Public APIs returning `Diagnostic` will need updates

## Example: Before and After

### Before (Current Diagnostic)
```rust
fn deploy_contract(&self, address: &str) -> Result<(), Diagnostic> {
    let parsed = parse_address(address)
        .map_err(|e| diagnosed_error!("unable to parse address: {}", e))?;
    
    do_deploy(parsed)
        .map_err(|e| diagnosed_error!("deployment failed: {}", e))
}
```

### After (With error-stack)
```rust
fn deploy_contract(&self, address: &str) -> Result<(), Report<EvmError>> {
    let parsed = parse_address(address)
        .change_context(EvmError::InvalidAddress)
        .attach_printable(format!("address: {}", address))
        .attach(ErrorDocumentation {
            help: "Ensure the address starts with 0x and contains 40 hex characters".into(),
            example: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f89590".into()),
            link: None,
        })?;
    
    do_deploy(parsed)
        .change_context(EvmError::ContractDeploymentFailed)
        .attach_printable("Check that the contract bytecode is valid")
        .attach(ActionContext {
            action_name: "deploy_contract".into(),
            namespace: "evm".into(),
            construct_id: self.id.clone(),
        })
}
```

## Recommendation

The error-stack library would significantly improve txtx's error reporting by:
1. Providing structured, context-rich errors
2. Enforcing good error handling practices
3. Offering better debugging capabilities
4. Standardizing error handling across the codebase

The migration effort is substantial but can be done incrementally, starting with new code and gradually updating existing error handling.