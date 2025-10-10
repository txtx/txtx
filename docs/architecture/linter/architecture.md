# Linter Architecture

## Overview

The txtx linter performs static analysis of runbooks and manifests, catching configuration errors before execution. It provides pre-execution validation similar to TypeScript's `tsc`, with multiple output formats for both human and machine consumption.

## Architecture Diagram

```mermaid
graph TB
    subgraph "Entry Point"
        CLI[txtx lint command]
    end

    subgraph "Workspace Discovery"
        WA[WorkspaceAnalyzer]
        WA --> |searches upward| Manifest[Find txtx.yml]
        WA --> |resolves paths| Runbooks[Locate runbooks]
    end

    subgraph "Validation Pipeline"
        Linter[Linter Engine]

        subgraph "Core Validation (txtx-core)"
            VC[ValidationContext]
            HCL[HCL Validator]
            MV[Manifest Validator]
            FB[File Boundary Mapper]
        end

        subgraph "Linter Rules (txtx-cli)"
            Rules[Rule Functions]
            Rules --> R1[undefined-input]
            Rules --> R2[naming-convention]
            Rules --> R3[cli-override]
            Rules --> R4[sensitive-data]
        end
    end

    subgraph "Multi-File Support"
        Combine[Concatenate Files]
        Track[Track Boundaries]
        Map[Map Error Locations]
    end

    subgraph "Output Formatting"
        Formatter[Formatter Engine]
        Formatter --> Stylish[Stylish - human]
        Formatter --> Compact[Compact - human]
        Formatter --> JSON[JSON - machine]
        Formatter --> Quickfix[Quickfix - IDE]
    end

    CLI --> WA
    WA --> Linter
    Linter --> VC
    VC --> HCL
    VC --> MV
    VC --> Rules

    Linter --> |multi-file runbook| Combine
    Combine --> Track
    Track --> VC
    VC --> |errors| Map
    Map --> FB
    FB --> |accurate locations| Formatter

    Linter --> |single file| VC
    VC --> |errors| Formatter

    style CLI fill:#e1f5ff
    style VC fill:#f96,stroke:#333,stroke-width:3px
    style Linter fill:#fff3e0
    style Formatter fill:#f3e5f5
```

## Validation Layers

The linter operates in three distinct layers:

### 1. HCL Validation (txtx-core)

**Purpose**: Syntax and semantic correctness

```mermaid
graph LR
    Input[Runbook Content] --> Parser[HCL Parser]
    Parser --> AST[Abstract Syntax Tree]
    AST --> Visitor[AST Visitor]

    Visitor --> |collect| Defs[Definitions]
    Visitor --> |collect| Refs[References]

    Defs --> Validate{Match?}
    Refs --> Validate

    Validate --> |missing| Errors[Undefined reference]
    Validate --> |circular| Errors2[Circular dependency]
    Validate --> |ok| Success[Valid]

    style Errors fill:#ffcccc
    style Errors2 fill:#ffcccc
    style Success fill:#ccffcc
```

**Checks:**

- Undefined variables, actions, flows
- Circular dependencies
- Invalid syntax
- Type mismatches

### 2. Manifest Validation (txtx-core)

**Purpose**: Environment and input validation

```mermaid
graph TB
    subgraph "Manifest Context"
        Env[Selected Environment]
        Global[Global Inputs]
        EnvInputs[Environment Inputs]
    end

    subgraph "Runbook Analysis"
        Extract[Extract input.* refs]
        FlowRefs[Extract flow.* refs]
    end

    Extract --> Check{Defined?}
    Env --> Check
    Global --> Check
    EnvInputs --> Check

    Check --> |no| Error1[Missing input error]
    Check --> |yes| Success1[Valid]

    FlowRefs --> FlowCheck{Flow defined?}
    FlowCheck --> |no| Error2[Missing flow input]
    FlowCheck --> |partial| Error3[Missing in some flows]
    FlowCheck --> |yes| Success2[Valid]

    Error2 --> RelLoc[Add related locations]
    Error3 --> RelLoc

    style Error1 fill:#ffcccc
    style Error2 fill:#ffcccc
    style Error3 fill:#ffcccc
    style Success1 fill:#ccffcc
    style Success2 fill:#ccffcc
```

**Checks:**

- Input defined in manifest
- Environment variables exist
- Flow inputs across multi-file runbooks
- Related locations for missing inputs

### 3. Linter Rules (txtx-cli)

**Purpose**: Style, conventions, and best practices

```mermaid
graph TB
    Context[ValidationContext] --> Rules{Run Rules}

    Rules --> R1[undefined-input]
    Rules --> R2[naming-convention]
    Rules --> R3[cli-override]
    Rules --> R4[sensitive-data]

    R1 --> |manifest context| Check1{Input exists?}
    Check1 --> |no| E1[Error: undefined]
    Check1 --> |yes| OK1[Pass]

    R2 --> Check2{Matches convention?}
    Check2 --> |no| W1[Warning: style]
    Check2 --> |yes| OK2[Pass]

    R3 --> Check3{CLI overrides env?}
    Check3 --> |yes| W2[Warning: override]
    Check3 --> |no| OK3[Pass]

    R4 --> Check4{Contains sensitive?}
    Check4 --> |yes| S1[Suggestion: vault]
    Check4 --> |no| OK4[Pass]

    style E1 fill:#ffcccc
    style W1 fill:#fff3cd
    style W2 fill:#fff3cd
    style S1 fill:#d1ecf1
    style OK1 fill:#ccffcc
    style OK2 fill:#ccffcc
    style OK3 fill:#ccffcc
    style OK4 fill:#ccffcc
```

**Rule Types:**

- **Errors**: Must be fixed (undefined inputs)
- **Warnings**: Should be fixed (naming, overrides)
- **Suggestions**: Consider fixing (sensitive data)

## Multi-File Runbook Validation

For runbooks spanning multiple files, the linter uses file boundary mapping to provide accurate error locations:

```mermaid
sequenceDiagram
    participant WA as WorkspaceAnalyzer
    participant Linter
    participant FBM as FileBoundaryMap
    participant Validator
    participant Result

    WA->>Linter: validate multi-file runbook

    Note over Linter: Concatenate files

    loop For each file
        Linter->>FBM: add_file(path, line_count)
        Linter->>Linter: append content
    end

    Linter->>Validator: validate(combined_content)
    Validator-->>Result: errors with combined line numbers

    Note over Result: Map to source files

    loop For each error
        Result->>FBM: map_line(combined_line)
        FBM-->>Result: (file_path, source_line)
        Result->>Result: update error location
    end

    loop For each related_location
        Result->>FBM: map_line(combined_line)
        FBM-->>Result: (file_path, source_line)
        Result->>Result: update related location
    end

    Result-->>Linter: errors with accurate locations

    Note over Linter: flows.tx:5:1 (not "multi-file:8:1")
```

**Benefits:**

1. **Shared State**: All files in runbook share flow/variable definitions
2. **Accurate Locations**: Errors show correct file:line:col
3. **Related Locations**: Cross-file references shown in context

**Example Output:**

```console
error: Flow 'deploy' missing input 'chain_id' flows.tx:5:1
  → Referenced here
    at deploy.tx:11:5
```

## Module Structure

### Flat Architecture (6 files, ~660 LOC)

```console
cli/linter/
├── mod.rs         # Public API, re-exports (50 lines)
├── config.rs      # LinterConfig struct (40 lines)
├── rules.rs       # All 4 validation rules (165 lines)
├── validator.rs   # Linter engine, IntoManifest trait (160 lines)
├── formatter.rs   # 4 output formats (130 lines)
└── workspace.rs   # Workspace discovery & runbook resolution (115 lines)
```

**Design Principles:**

- Single-level module structure
- Function pointers over trait objects (zero-cost)
- Cow<str> for static strings (zero allocation)
- Data-driven configuration (const arrays)
- Clear separation of concerns

### Performance Characteristics

| Aspect | Implementation | Benefit |
|--------|---------------|---------|
| Rules | `fn(&ValidationContext) -> Option<ValidationIssue>` | Stack allocation, no heap |
| Strings | `Cow::Borrowed("static")` | Zero allocation |
| Patterns | `const SENSITIVE_PATTERNS: &[&str]` | Compile-time data |
| Lifetimes | `ValidationContext<'env, 'content>` | Explicit borrowing |

## Validation Flow

### Complete Validation Pipeline

```mermaid
flowchart TD
    Start([txtx lint runbook]) --> Discover

    Discover[Workspace Discovery] --> CheckManifest{Manifest found?}
    CheckManifest --> |no| SearchUp[Search parent dirs]
    SearchUp --> |found| LoadManifest
    SearchUp --> |git root| Error1[Error: No manifest]
    CheckManifest --> |yes| LoadManifest[Load Manifest]

    LoadManifest --> ResolveRunbook{Runbook path?}
    ResolveRunbook --> |explicit| UseExplicit[Use provided path]
    ResolveRunbook --> |none| SearchStandard[Check standard locations]

    UseExplicit --> CheckExists{Exists?}
    SearchStandard --> CheckExists
    CheckExists --> |no| Error2[Error: Not found]
    CheckExists --> |yes| CheckType{Multi-file?}

    CheckType --> |directory| MultiFile[Load all .tx files]
    CheckType --> |single| SingleFile[Load file]

    MultiFile --> Concatenate[Concatenate with boundaries]
    Concatenate --> BuildMap[Build FileBoundaryMap]
    BuildMap --> ValidateCombined

    SingleFile --> ValidateSingle[Validate single file]

    subgraph "Validation"
        ValidateCombined[Validate Combined]
        ValidateSingle

        ValidateCombined --> HCLParse[HCL Parse]
        ValidateSingle --> HCLParse

        HCLParse --> |syntax error| HCLDiag[HCL Diagnostics]
        HCLParse --> |ok| ASTVisit[AST Visitor]

        ASTVisit --> CollectItems[Collect Definitions & Refs]
        CollectItems --> CheckCircular{Circular deps?}
        CheckCircular --> |yes| CircError[Circular dependency error]
        CheckCircular --> |no| CheckUndef{Undefined refs?}
        CheckUndef --> |yes| UndefError[Undefined reference error]
        CheckUndef --> |no| ManifestCheck

        ManifestCheck[Manifest Validation] --> CheckInputs{Inputs defined?}
        CheckInputs --> |no| InputError[Input error + related locations]
        CheckInputs --> |yes| FlowCheck{Flow inputs valid?}
        FlowCheck --> |missing| FlowError[Flow error + related locations]
        FlowCheck --> |ok| RunRules

        RunRules[Run Linter Rules] --> Aggregate
    end

    HCLDiag --> MapErrors
    CircError --> MapErrors
    UndefError --> MapErrors
    InputError --> MapErrors
    FlowError --> MapErrors
    Aggregate --> MapErrors

    MapErrors{Multi-file?} --> |yes| MapToSource[Map to source files]
    MapErrors --> |no| Format
    MapToSource --> Format[Format Results]

    Format --> Output{Format?}
    Output --> |stylish| Stylish[Human-readable output]
    Output --> |compact| Compact[Condensed output]
    Output --> |json| JSON[Machine-readable JSON]
    Output --> |quickfix| Quickfix[IDE quickfix format]

    Stylish --> End([Exit with status])
    Compact --> End
    JSON --> End
    Quickfix --> End
    Error1 --> End
    Error2 --> End

    style Error1 fill:#ffcccc
    style Error2 fill:#ffcccc
    style HCLDiag fill:#ffcccc
    style CircError fill:#ffcccc
    style UndefError fill:#ffcccc
    style InputError fill:#ffcccc
    style FlowError fill:#ffcccc
    style End fill:#e1f5ff
```

## Output Formats

The linter supports multiple output formats for different use cases:

### Stylish (Human-readable)

```console
error: Flow 'deploy' missing input 'chain_id' flows.tx:5:1
  → Referenced here
    at deploy.tx:11:5

warning: Input 'api_key' uses CLI override main.tx:8:1
  The CLI input '--input api_key=value' overrides the manifest environment value
```

### Compact (Condensed)

```console
flows.tx:5:1 error Flow 'deploy' missing input 'chain_id'
main.tx:8:1 warning Input 'api_key' uses CLI override
```

### JSON (Machine-readable)

```json
{
  "errors": [
    {
      "message": "Flow 'deploy' missing input 'chain_id'",
      "file": "flows.tx",
      "line": 5,
      "column": 1,
      "related_locations": [
        {"file": "deploy.tx", "line": 11, "column": 5, "message": "Referenced here"}
      ]
    }
  ]
}
```

### Quickfix (IDE integration)

```console
flows.tx:5:1: error: Flow 'deploy' missing input 'chain_id'
deploy.tx:11:5: note: Referenced here
```

## Integration Points

### CLI Integration

```console
txtx lint [RUNBOOK] [OPTIONS]
  --manifest-path PATH    Explicit manifest location
  --env ENV              Environment to validate against
  --input KEY=VALUE      CLI input overrides (triggers warnings)
  --format FORMAT        Output format (stylish|compact|json|quickfix)
  --gen-cli              Generate CLI command from inputs
```

### LSP Integration

The linter is used by the LSP for real-time diagnostics:

```rust
// LSP calls linter for workspace diagnostics
let result = linter.validate_content(
    &combined_content,
    &manifest,
    environment,
    addon_specs,
)?;

// Map errors to source files
result.map_errors_to_source_files(&boundary_map);

// Convert to LSP diagnostics
let diagnostics = result.errors.iter()
    .map(|e| to_lsp_diagnostic(e))
    .collect();
```

## Key Features

1. **Multi-file Validation**: Validates entire runbooks with shared state
2. **File Boundary Mapping**: Accurate error locations across files
3. **Related Locations**: Shows cross-file references in error context
4. **Flow Validation**: Validates flow inputs across runbook files
5. **Environment Context**: Validates against specific manifest environments
6. **Multiple Formats**: Human and machine-readable output
7. **Workspace Discovery**: Automatic manifest location
8. **Zero-cost Abstractions**: Function pointers, no heap allocation

## Related Documentation

- [Validation Architecture](../../developer/VALIDATION_ARCHITECTURE.md) - Deep dive into validation system
- [Linter User Guide](../../user/linter-guide.md) - Usage and examples
- [ADR 003: Capture Everything Pattern](../../adr/003-capture-everything-filter-later-pattern.md) - Validation approach
- [ADR 004: Visitor Strategy Pattern](../../adr/004-visitor-strategy-pattern-with-readonly-iterators.md) - AST traversal
