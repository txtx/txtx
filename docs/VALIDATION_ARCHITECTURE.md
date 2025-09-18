# Validation Architecture

This document describes the validation system architecture in txtx, including the recent refactoring that introduced `ValidationContext` and moved manifest validation from CLI to core.

## Overview

The txtx validation system provides multiple levels of validation:

1. **HCL Syntax Validation** - Validates the runbook syntax
2. **Semantic Validation** - Checks references, types, and addon specifications
3. **Manifest Validation** - Validates environment variables and inputs against a workspace manifest
4. **Doctor Validation** - Enhanced validation with additional rules and checks

## Component Diagram

```mermaid
graph TB
    subgraph "txtx-test-utils"
        RB[RunbookBuilder]
        SV[SimpleValidator]
        AR[AddonRegistry]
    end
    
    subgraph "txtx-core::validation"
        VC[ValidationContext]
        HV[HCL Validator]
        MV[Manifest Validator]
        DR[Doctor Rules]
        AS[Addon Specifications]
        VT[Validation Types]
    end
    
    subgraph "txtx-cli::doctor"
        DA[Doctor Analyzer]
        DI[Doctor Inputs]
    end
    
    subgraph "txtx-addon-kit"
        AK[Command Specs]
    end
    
    RB -->|uses| SV
    SV -->|creates| VC
    SV -->|gets specs| AR
    AR -->|loads| AK
    
    VC -->|delegates to| HV
    VC -->|delegates to| MV
    MV -->|uses| DR
    HV -->|uses| AS
    
    DA -->|uses| VC
    DA -->|wraps| DI
    
    style VC fill:#f96,stroke:#333,stroke-width:4px
    style RB fill:#9cf,stroke:#333,stroke-width:2px
    style DA fill:#fc9,stroke:#333,stroke-width:2px
```

## Dependency Diagram

```mermaid
graph BT
    AK[txtx-addon-kit]
    TC[txtx-core]
    TTU[txtx-test-utils]
    TCLI[txtx-cli]
    
    TC --> AK
    TTU --> TC
    TTU --> AK
    TCLI --> TC
    TCLI --> AK
    TCLI -.->|doctor ext trait| TTU
    
    subgraph "Key Dependencies"
        TC -.- VC[ValidationContext]
        TC -.- MV[ManifestValidator]
        TC -.- DR[DoctorRules]
    end
    
    style TC fill:#f96,stroke:#333,stroke-width:4px
    style VC fill:#ffa,stroke:#333,stroke-width:2px
    style MV fill:#ffa,stroke:#333,stroke-width:2px
    style DR fill:#ffa,stroke:#333,stroke-width:2px
```

## Validation Workflow

```mermaid
sequenceDiagram
    participant User
    participant RB as RunbookBuilder
    participant SV as SimpleValidator
    participant VC as ValidationContext
    participant HV as HCL Validator
    participant MV as Manifest Validator
    participant DR as Doctor Rules
    
    User->>RB: build runbook
    User->>RB: set environment
    User->>RB: validate()
    
    alt Has manifest or environment set
        RB->>SV: validate_content_with_manifest()
        SV->>VC: new(content, file_path)
        SV->>VC: with_manifest(manifest)
        SV->>VC: with_environment(env)
        SV->>VC: with_addon_specs(specs)
        
        SV->>VC: validate_full()
        VC->>HV: validate_with_hcl()
        HV-->>VC: input_refs
        
        VC->>MV: validate_manifest()
        MV->>DR: check rules
        DR-->>MV: validation outcomes
        MV-->>VC: errors/warnings
        
        VC-->>SV: ValidationResult
        SV-->>RB: ValidationResult
    else No manifest and no environment
        RB->>SV: validate_content()
        SV->>HV: validate_with_hcl()
        HV-->>SV: ValidationResult
        SV-->>RB: ValidationResult
    end
    
    RB-->>User: ValidationResult
```

## Validation Modes Comparison

```mermaid
graph LR
    subgraph "HCL-Only Validation"
        H1[Parse HCL]
        H2[Check Syntax]
        H3[Validate Addons]
        H1 --> H2 --> H3
    end
    
    subgraph "Manifest Validation"
        M1[HCL Validation]
        M2[Load Manifest]
        M3[Check Env Vars]
        M4[Apply Rules]
        M1 --> M2 --> M3 --> M4
    end
    
    subgraph "Doctor Validation"
        D1[Manifest Validation]
        D2[Enhanced Rules]
        D3[Cross-References]
        D4[Best Practices]
        D1 --> D2 --> D3 --> D4
    end
    
    style M3 fill:#f96,stroke:#333,stroke-width:2px
    style D2 fill:#fc9,stroke:#333,stroke-width:2px
```

## Key Design Decisions

### 1. ValidationContext Introduction
The `ValidationContext` consolidates all validation parameters into a single object:
- Reduces parameter passing complexity
- Enables cleaner extension with new validation features
- Provides caching for computed values (e.g., effective inputs)

### 2. Manifest Validation Requirements
Manifest validation **requires** an environment to be specified:
- Without an environment, only "defaults" can be validated (partial scenario)
- This prevents false confidence from incomplete validation
- RunbookBuilder enforces this by requiring both manifest AND environment

### 3. Separation of Concerns
- **txtx-core**: Core validation logic (HCL, manifest, rules)
- **txtx-cli**: Doctor-specific analysis and enhanced validation
- **txtx-test-utils**: Test builder API and validation helpers

### 4. Extensible Rules System
The `ManifestValidationRule` trait allows:
- Core rules in txtx-core
- Doctor-specific rules in txtx-core (used by CLI)
- Custom rules for specific use cases

## ValidationContext API

```rust
// Create context with builder pattern
let mut context = ValidationContext::new(content, "test.tx")
    .with_manifest(manifest)
    .with_environment("production")
    .with_cli_inputs(vec![("key", "value")])
    .with_addon_specs(specs);

// Run full validation pipeline
context.validate_full(&mut result)?;

// Or run specific validation phases
context.validate_hcl(&mut result)?;
context.validate_manifest(config, &mut result);
```

## Rule Implementation Example

```rust
pub struct SensitiveDataRule;

impl ManifestValidationRule for SensitiveDataRule {
    fn check(&self, context: &ManifestValidationContext) -> ValidationOutcome {
        if context.input_name.contains("key") || 
           context.input_name.contains("secret") {
            if let Some(value) = context.effective_inputs.get(context.input_name) {
                if !value.starts_with("$") && !value.contains("vault") {
                    return ValidationOutcome::Warning {
                        message: format!("Sensitive data in '{}' may be exposed", context.input_name),
                        suggestion: Some("Consider using environment variables or a secrets manager".to_string()),
                    };
                }
            }
        }
        ValidationOutcome::Pass
    }
}
```

## Future Enhancements

1. **Async Validation** - Support for async validation rules
2. **Parallel Rule Execution** - Run independent rules concurrently
3. **Rule Priorities** - Allow rules to specify execution order
4. **Validation Caching** - Cache validation results for unchanged content
5. **Custom Rule Plugins** - Dynamic loading of validation rules
