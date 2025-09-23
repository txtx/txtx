# 2. Integrate HCL Validation into LSP

Date: 2025-01-15

## Status

Accepted

## Context

The txtx project uses HCL-like syntax for its runbook files (`.tx` extension). Currently, the LSP implementation provides custom validation, but we discovered that the underlying HCL parser already provides rich diagnostic information including:

- Syntax errors with precise locations
- Detailed error messages
- Suggestions for fixes
- Multiple diagnostics per file

The current LSP validation duplicates some of this work and may miss diagnostics that the HCL parser could provide directly.

## Decision

We will integrate the HCL parser's native validation and diagnostics directly into the LSP implementation, leveraging the parser's built-in error reporting capabilities rather than implementing custom validation logic.

## Consequences

### Positive

- **More comprehensive diagnostics**: The HCL parser catches more syntax issues than custom validation
- **Consistent error messages**: Users get the same error messages whether using CLI or LSP
- **Reduced code duplication**: No need to reimplement validation that the parser already provides
- **Better maintainability**: Updates to the HCL parser automatically improve LSP diagnostics
- **Richer error information**: HCL diagnostics include suggestions and context

### Negative

- **Less control over error formatting**: Must work within the HCL diagnostic format
- **Potential for noise**: HCL parser may report issues that are valid in txtx context
- **Dependency on HCL parser behavior**: Changes in the parser could affect LSP behavior

## Implementation

### Phase 1: Extract HCL Diagnostics

1. Modify the parser to expose HCL diagnostic information
2. Convert HCL diagnostics to LSP diagnostic format
3. Preserve all diagnostic metadata (severity, suggestions, ranges)

### Phase 2: Integration with LSP

1. Update the LSP validation handler to use HCL diagnostics
2. Map HCL diagnostic severity to LSP diagnostic severity
3. Include HCL suggestions in LSP code actions where applicable

### Phase 3: Custom Validation Layer

1. Keep a thin custom validation layer for txtx-specific rules
2. Merge HCL diagnostics with custom diagnostics
3. Ensure no duplicate diagnostics are reported

## Example

HCL parser diagnostic:

```console
Error: Invalid block definition
  on main.tx line 15:
  15: addon "evm" {
  
A block definition must have block content delimited by "{" and "}", 
starting on the same line as the block header.
```

Converted to LSP diagnostic:

```json
{
  "range": {
    "start": { "line": 14, "character": 0 },
    "end": { "line": 14, "character": 13 }
  },
  "severity": 1,
  "source": "txtx-hcl",
  "message": "Invalid block definition: A block definition must have block content delimited by \"{\" and \"}\", starting on the same line as the block header."
}
```

## Alternatives Considered

### Keep Custom Validation Only

- **Pros**: Full control over validation logic and error messages
- **Cons**: Duplicates work, misses HCL parser insights, more code to maintain
- **Rejected**: The HCL parser already provides superior diagnostics

### Replace All Validation with HCL Parser

- **Pros**: Simplest implementation, no custom code
- **Cons**: Cannot validate txtx-specific semantics
- **Rejected**: Some txtx rules go beyond HCL syntax

## References

- [HCL diagnostic documentation](https://github.com/hashicorp/hcl/tree/main/hclsyntax)
- [LSP specification for diagnostics](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic)
- Current txtx validation implementation in `txtx-cli/src/cli/lsp/validation/`
