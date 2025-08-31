# Error-Stack Preservation Architecture

## Overview

This document describes the architecture for preserving strongly-typed error-stack errors (`Report<EvmError>`) when they are converted to generic `Diagnostic` types for txtx-core. This pattern enables:

1. **Type-safe error testing**: Tests can access the original strongly-typed errors
2. **Migration path**: Other addons can adopt error-stack gradually
3. **Rich error context**: Full error chains with attachments are preserved
4. **Backward compatibility**: Existing code continues to work

## Problem Statement

The txtx-core runtime uses generic `Diagnostic` types for error reporting across all addons. However, the EVM addon uses error-stack with strongly-typed `Report<EvmError>` that provides rich error context and type safety. When converting between these types, we lose:

- Type information (specific error variants)
- Error attachments and context
- Ability to pattern match on specific errors in tests

## Solution: Source Error Preservation

### Core Concept

Extend `Diagnostic` with an optional `source_error` field that can hold the original typed error as a trait object:

```rust
pub struct Diagnostic {
    // ... existing fields ...
    
    /// Original error preserved for addons using error-stack
    /// Uses Arc internally via error-stack's Report, making clones cheap
    #[serde(skip)]
    pub source_error: Option<Box<dyn std::any::Any + Send + Sync>>,
}
```

### Why Clone?

The error-stack `Report<C>` type uses `Arc` internally, making it cheaply cloneable (just increments reference count). This is by design - error-stack expects Reports to be cloned when needed for:
- Multiple formatting operations
- Logging at different levels
- Storing in multiple locations

### Conversion Pattern

```rust
impl From<EvmErrorReport> for Diagnostic {
    fn from(wrapper: EvmErrorReport) -> Self {
        let report = wrapper.0;
        
        // Create diagnostic with main error message
        let mut diagnostic = Diagnostic::error_from_string(report.to_string());
        
        // Add full error chain as documentation
        let error_chain = format!("{:?}", report.clone()); // Cheap clone
        diagnostic.documentation = Some(format!("Full error context:\n{}", error_chain));
        
        // Preserve the original Report<EvmError>
        diagnostic.source_error = Some(Box::new(report));
        
        diagnostic
    }
}
```

### Extraction Pattern

```rust
impl Diagnostic {
    /// Try to downcast the source error to a specific type
    pub fn downcast_source<T: std::any::Any>(&self) -> Option<&T> {
        self.source_error
            .as_ref()
            .and_then(|e| e.downcast_ref::<T>())
    }
}
```

## Usage in Tests

### Creating Errors

```rust
// In EVM addon action
let report = Report::new(EvmError::Transaction(
    TransactionError::InsufficientFunds { required: 1000, available: 0 }
))
.attach_printable("Account has no funds")
.attach(TransactionContext { ... });

// Convert to Diagnostic (preserves Report)
let diagnostic = report_to_diagnostic(report);
```

### Testing Errors

```rust
// In test
let result = harness.execute_runbook();

if let Err(report) = result {
    // Direct access to strongly-typed error
    assert!(matches!(
        report.current_context(),
        EvmError::Transaction(TransactionError::InsufficientFunds { .. })
    ));
}
```

### Extracting from Diagnostics

```rust
// When receiving Vec<Diagnostic> from txtx-core
for diagnostic in diagnostics {
    if let Some(report) = diagnostic.downcast_source::<Report<EvmError>>() {
        // Access the original strongly-typed error
        match report.current_context() {
            EvmError::Transaction(TransactionError::InsufficientFunds { required, available }) => {
                println!("Need {} but only have {}", required, available);
            }
            _ => {}
        }
    }
}
```

## Migration Guide for Other Addons

1. **Define your error types** using error-stack:
   ```rust
   #[derive(Debug)]
   pub enum MyAddonError {
       NetworkError(String),
       ValidationError { field: String, reason: String },
   }
   ```

2. **Create Report wrapper** for conversion:
   ```rust
   pub struct MyAddonErrorReport(pub Report<MyAddonError>);
   ```

3. **Implement conversion** to Diagnostic:
   ```rust
   impl From<MyAddonErrorReport> for Diagnostic {
       fn from(wrapper: MyAddonErrorReport) -> Self {
           let report = wrapper.0;
           let mut diagnostic = Diagnostic::error_from_string(report.to_string());
           diagnostic.source_error = Some(Box::new(report));
           diagnostic
       }
   }
   ```

4. **Use in tests** with downcasting:
   ```rust
   if let Some(report) = diagnostic.downcast_source::<Report<MyAddonError>>() {
       // Test with strongly-typed errors
   }
   ```

## Benefits

1. **Type Safety**: Tests can use pattern matching on specific error types
2. **Rich Context**: Full error-stack chains with attachments are preserved
3. **Debugging**: Complete error information available in development
4. **Gradual Adoption**: Addons can migrate to error-stack at their own pace
5. **Performance**: Uses Arc reference counting, minimal overhead
6. **Backward Compatible**: Existing code continues to work unchanged

## Performance Considerations

- `Report<T>` uses `Arc` internally - cloning is just reference count increment
- `Box<dyn Any>` adds one allocation per error (acceptable for error paths)
- No impact on success paths
- Serialization skips `source_error` field to avoid overhead

## Future Enhancements

1. **Type Registry**: Register error types for better introspection
2. **Error Chains**: Preserve full chain of errors with causes
3. **Structured Extraction**: Helper methods for common error patterns
4. **Cross-Addon Errors**: Standardized error types for common failures