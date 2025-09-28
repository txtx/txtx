# txtx LSP Use Case Diagram

This document provides use case diagrams illustrating how different actors interact with the txtx Language Server.

## Primary Use Case Diagram

```mermaid
graph TB
    subgraph Actors
        Dev[Developer/User]
        Editor[Code Editor<br/>VS Code, Neovim, etc.]
        ExtPlugin[Editor Extension/<br/>Language Client Plugin]
    end

    subgraph "txtx Language Server"
        LSP[LSP Server Core]

        subgraph "Document Management"
            UC1[UC1: Open Document]
            UC2[UC2: Edit Document]
            UC3[UC3: Close Document]
        end

        subgraph "Code Intelligence"
            UC4[UC4: Get Diagnostics]
            UC5[UC5: Navigate to Definition]
            UC6[UC6: View Hover Info]
            UC7[UC7: Get Completions]
        end

        subgraph "Environment Management"
            UC8[UC8: List Environments]
            UC9[UC9: Switch Environment]
            UC10[UC10: Validate in Context]
        end

        subgraph "Validation System"
            UC11[UC11: HCL Syntax Check]
            UC12[UC12: Run Linter Rules]
            UC13[UC13: Multi-file Validation]
        end
    end

    subgraph "Backend Systems"
        WS[Workspace State]
        Linter[Linter Engine]
        HCL[HCL Parser]
        Manifest[Manifest Parser]
        FuncReg[Function Registry]
    end

    Dev -->|types code| Editor
    Editor -->|LSP protocol| ExtPlugin
    ExtPlugin -->|JSON-RPC| LSP

    LSP --> UC1
    LSP --> UC2
    LSP --> UC3
    LSP --> UC4
    LSP --> UC5
    LSP --> UC6
    LSP --> UC7
    LSP --> UC8
    LSP --> UC9
    LSP --> UC10
    LSP --> UC11
    LSP --> UC12
    LSP --> UC13

    UC1 --> WS
    UC2 --> WS
    UC3 --> WS
    UC4 --> Linter
    UC4 --> HCL
    UC5 --> Manifest
    UC6 --> FuncReg
    UC6 --> Manifest
    UC7 --> Manifest
    UC8 --> Manifest
    UC8 --> WS
    UC9 --> WS
    UC10 --> Linter
    UC11 --> HCL
    UC12 --> Linter
    UC13 --> Linter
    UC13 --> Manifest

    style Dev fill:#e1f5ff
    style Editor fill:#e1f5ff
    style ExtPlugin fill:#e1f5ff
    style LSP fill:#fff3e0
    style WS fill:#f3e5f5
    style Linter fill:#f3e5f5
    style HCL fill:#f3e5f5
    style Manifest fill:#f3e5f5
    style FuncReg fill:#f3e5f5
```

## Detailed Use Cases

### UC1: Open Document (textDocument/didOpen)

```mermaid
graph LR
    A[Developer opens<br/>txtx file] --> B[Editor sends<br/>didOpen notification]
    B --> C[LSP: DocumentSyncHandler<br/>stores document]
    C --> D[LSP: Workspace<br/>caches content + version]
    D --> E[LSP: DiagnosticsHandler<br/>validates document]
    E --> F{Is runbook?}
    F -->|Yes| G[Find manifest]
    F -->|No| K[No diagnostics]
    G --> H{Multi-file?}
    H -->|Yes| I[Load all files<br/>from directory]
    H -->|No| J[Validate single file]
    I --> L[Run HCL parser<br/>+ Linter rules]
    J --> L
    L --> M[Convert to<br/>LSP Diagnostics]
    M --> N[Send publishDiagnostics<br/>to editor]
    N --> O[Editor shows<br/>errors/warnings]
```

**Actors**: Developer, Editor, LSP Server
**Preconditions**:
- LSP server initialized
- File is `.tx` or `.yml` format
**Flow**:
1. Developer opens file in editor
2. Editor sends `textDocument/didOpen` notification
3. DocumentSyncHandler stores document in workspace state
4. DiagnosticsHandler validates the document
5. Results sent back as diagnostics
**Postconditions**: Document tracked, diagnostics displayed

---

### UC2: Edit Document (textDocument/didChange)

```mermaid
graph LR
    A[Developer types<br/>in editor] --> B[Editor sends<br/>didChange notification]
    B --> C[LSP: DocumentSyncHandler<br/>updates content]
    C --> D[Workspace: Increment<br/>version number]
    D --> E[LSP: DiagnosticsHandler<br/>re-validates]
    E --> F{Multi-file<br/>runbook?}
    F -->|Yes| G[Reload all files<br/>in directory]
    F -->|No| H[Validate current<br/>content]
    G --> I[Run validation]
    H --> I
    I --> J[Send updated<br/>diagnostics]
    J --> K[Editor updates<br/>error markers]
```

**Actors**: Developer, Editor
**Preconditions**: Document is open
**Flow**:
1. Developer makes changes
2. Editor sends full content in `didChange`
3. DocumentSyncHandler updates workspace
4. Automatic re-validation triggered
5. Fresh diagnostics sent
**Postconditions**: Document state synchronized, validation current

---

### UC4: Get Diagnostics (Validation)

```mermaid
graph TB
    Start[Validation<br/>Requested] --> Check{Document<br/>Type}
    Check -->|Runbook .tx| RunbookFlow
    Check -->|Manifest .yml| ManifestFlow
    Check -->|Other| NoValidation[Return empty]

    RunbookFlow --> FindManifest[Find associated<br/>txtx.yml manifest]
    FindManifest --> MultiCheck{Multi-file<br/>runbook?}

    MultiCheck -->|Yes| LoadAll[Load all .tx files<br/>in directory]
    MultiCheck -->|No| SingleFile[Use current file]

    LoadAll --> Combine[Combine files with<br/>line markers]
    Combine --> Parse
    SingleFile --> Parse[HCL Parser]

    Parse --> SyntaxCheck{Syntax<br/>OK?}
    SyntaxCheck -->|No| SyntaxErr[Return syntax errors<br/>with positions]
    SyntaxCheck -->|Yes| AST[Generate AST]

    AST --> LinterRules[Run Linter Rules]

    subgraph "Linter Rules"
        R1[undefined-input]
        R2[cli-override]
        R3[type-check]
        R4[semantic-validation]
    end

    LinterRules --> R1
    LinterRules --> R2
    LinterRules --> R3
    LinterRules --> R4

    R1 --> Collect[Collect violations]
    R2 --> Collect
    R3 --> Collect
    R4 --> Collect

    Collect --> Convert[Convert to<br/>LSP Diagnostics]
    SyntaxErr --> Convert

    Convert --> MapLines{Multi-file?}
    MapLines -->|Yes| MapToFile[Map line numbers<br/>to source files]
    MapLines -->|No| Send
    MapToFile --> FilterFile[Filter diagnostics<br/>for current file]
    FilterFile --> Send[Send diagnostics<br/>to editor]

    ManifestFlow --> ValidateYAML[Validate YAML syntax]
    ValidateYAML --> Send
    NoValidation --> End[End]
    Send --> End
```

**Actors**: LSP Server, Linter, HCL Parser
**Purpose**: Provide real-time validation feedback
**Features**:
- Syntax validation (HCL parser errors)
- Semantic validation (linter rules)
- Environment-aware checking
- Multi-file runbook support

---

### UC5: Navigate to Definition (textDocument/definition)

```mermaid
graph LR
    A[Developer Ctrl+Click<br/>on input.variable] --> B[Editor sends<br/>definition request]
    B --> C[EnhancedDefinitionHandler<br/>parses cursor position]
    C --> D{Pattern<br/>match?}
    D -->|input.XXX| E[Extract variable name]
    D -->|No match| F[Return null]
    E --> G[Find manifest<br/>for runbook]
    G --> H[Search manifest YAML<br/>for variable definition]
    H --> I{Found?}
    I -->|Yes| J[Create Location with<br/>manifest URI + line]
    I -->|No| F
    J --> K[Editor jumps to<br/>manifest definition]
```

**Actors**: Developer, Editor
**Trigger**: Developer invokes "Go to Definition" on `input.variable`
**Flow**:
1. Editor sends cursor position
2. Handler extracts `input.` reference
3. Searches manifest environments
4. Returns location or null
**Result**: Editor navigates to variable definition in manifest

---

### UC6: View Hover Information (textDocument/hover)

```mermaid
graph TB
    Start[Developer hovers<br/>over symbol] --> Editor[Editor sends<br/>hover request]
    Editor --> Handler[HoverHandler<br/>processes request]
    Handler --> Extract[Extract symbol<br/>at position]

    Extract --> CheckType{Symbol<br/>Type?}

    CheckType -->|namespace::function| FuncFlow
    CheckType -->|namespace::action| ActionFlow
    CheckType -->|namespace::signer| SignerFlow
    CheckType -->|input.variable| InputFlow
    CheckType -->|None| ReturnNull[Return null]

    FuncFlow --> FuncReg[Function Registry<br/>lookup]
    FuncReg --> FuncDoc[Return function<br/>documentation]
    FuncDoc --> BuildHover

    ActionFlow --> ActionReg[Action Registry<br/>lookup]
    ActionReg --> ActionDoc[Return action<br/>documentation]
    ActionDoc --> BuildHover

    SignerFlow --> SignerCheck{Static or<br/>Environment?}
    SignerCheck -->|Static| StaticSigner[Return addon<br/>signer docs]
    SignerCheck -->|Environment| EnvSigner[Generate dynamic<br/>signer info]
    StaticSigner --> BuildHover
    EnvSigner --> BuildHover

    InputFlow --> GetEnv[Get current<br/>environment]
    GetEnv --> GetManifest[Get manifest]
    GetManifest --> Resolve[EnvironmentResolver:<br/>resolve_value]
    Resolve --> CheckValue{Value<br/>found?}

    CheckValue -->|Yes| ShowValue[Show:<br/>- Current value<br/>- Source environment<br/>- Other definitions]
    CheckValue -->|No| CheckOther{Defined<br/>elsewhere?}

    CheckOther -->|Yes| ShowWarning[Warning: Not in current env<br/>Show available environments]
    CheckOther -->|No| ShowError[Error: Not defined<br/>Suggest adding to manifest]

    ShowValue --> BuildHover
    ShowWarning --> BuildHover
    ShowError --> BuildHover

    BuildHover[Build Markdown<br/>hover content]
    BuildHover --> Return[Return Hover<br/>to editor]
    Return --> Display[Editor displays<br/>hover popup]
    ReturnNull --> End[End]
    Display --> End
```

**Actors**: Developer, Editor, LSP Server
**Types of Hover Info**:

1. **Functions** (`std::encode_hex`): Shows function signature and documentation
2. **Actions** (`evm::deploy_contract`): Shows action parameters and description
3. **Signers** (`bitcoin::alice`): Shows signer type and environment info
4. **Inputs** (`input.api_key`):
   - Shows current value in active environment
   - Warns if not defined in current environment
   - Lists other environments where defined
5. **Debug Commands** (`input.dump_txtx_state`): Special diagnostic info

---

### UC7: Get Completions (textDocument/completion)

```mermaid
graph LR
    A[Developer types<br/>'input.'] --> B[Editor sends<br/>completion request]
    B --> C{Async<br/>handling}
    C --> D[CompletionHandler<br/>on tokio runtime]
    D --> E[Check if after<br/>'input.' trigger]
    E --> F{Is after<br/>input.?}
    F -->|No| G[Return null]
    F -->|Yes| H[Get manifest<br/>for runbook]
    H --> I[Collect input keys<br/>from all environments]
    I --> J[Build CompletionItem<br/>list with type VARIABLE]
    J --> K[Return to editor<br/>via async channel]
    K --> L[Editor shows<br/>completion menu]
```

**Actors**: Developer, Editor
**Trigger**: User types `input.` or invokes completion
**Features**:
- Trigger character: `.`
- Runs asynchronously (non-blocking)
- Shows all available inputs across environments
**Result**: Dropdown list of available input variables

---

### UC8: List Environments (workspace/environments)

```mermaid
graph TB
    Start[Extension requests<br/>environments] --> Handler[WorkspaceHandler<br/>get_environments]

    Handler --> Collect1[Collect from<br/>open documents]
    Collect1 --> Parse1[Parse *.env.tx<br/>filenames]

    Handler --> Collect2[Collect from<br/>manifest]
    Collect2 --> Parse2[Parse environments<br/>section]

    Handler --> Check{Enough<br/>found?}
    Check -->|No| Scan[Scan workspace<br/>for .tx files]
    Check -->|Yes| Merge

    Scan --> FileScanner[FileScanner:<br/>find_tx_files]
    FileScanner --> Parse3[Extract environment<br/>from each file]
    Parse3 --> Merge[Merge all results]

    Merge --> Filter[Filter out 'global'<br/>Sort alphabetically]
    Filter --> Return[Return environment<br/>list to extension]
    Return --> UI[Extension shows<br/>environment picker]
```

**Actors**: Editor Extension, LSP Server
**Purpose**: Populate environment selector UI
**Sources**:
1. Open document filenames (*.{env}.tx)
2. Manifest environments section
3. Workspace file scan (if needed)
**Result**: List like `["sepolia", "mainnet", "testnet"]`

---

### UC9: Switch Environment (workspace/setEnvironment)

```mermaid
graph LR
    A[User selects<br/>environment in UI] --> B[Extension sends<br/>setEnvironment notification]
    B --> C[WorkspaceHandler<br/>updates state]
    C --> D[Set current_environment<br/>in workspace]
    D --> E[Get all open<br/>document URIs]
    E --> F{For each<br/>document}
    F --> G[DiagnosticsHandler:<br/>get_diagnostics_with_env]
    G --> H[Re-validate with<br/>new environment]
    H --> I[Send updated<br/>diagnostics]
    I --> F
    F --> J[All documents<br/>re-validated]
    J --> K[Editor updates<br/>all error markers]
```

**Actors**: Developer, Extension, LSP Server
**Flow**:
1. User selects environment from dropdown
2. Extension sends custom notification
3. Server updates global environment state
4. **All open documents re-validated** in new context
5. Fresh diagnostics sent for each document
**Impact**: Validation now checks against selected environment's inputs

---

### UC10: Validate in Context (Environment-Aware)

```mermaid
graph TB
    Start[Validation with<br/>environment context] --> GetEnv[Get current<br/>environment]
    GetEnv --> GetManifest[Load manifest]
    GetManifest --> Parse[Parse runbook]
    Parse --> ExtractInputs[Extract input.XXX<br/>references]

    ExtractInputs --> Check{For each<br/>input ref}
    Check --> Resolve[EnvironmentResolver:<br/>check if defined]

    Resolve --> InCurrent{In current<br/>environment?}
    InCurrent -->|No| CheckGlobal{In global<br/>environment?}
    InCurrent -->|Yes| Valid[OK]

    CheckGlobal -->|Yes| Inherited[OK - Inherited<br/>from global]
    CheckGlobal -->|No| Error[ERROR:<br/>Undefined input]

    Error --> CreateDiag[Create diagnostic:<br/>'input.XXX not defined<br/>in environment YYY']

    Valid --> Check
    Inherited --> Check
    CreateDiag --> Check
    Check --> Done[Validation complete]
```

**Purpose**: Ensure runbooks are valid for selected environment
**Key Rule**: `undefined-input` linter rule
**Behavior**:
- Checks each `input.` reference
- Resolves against current environment + global fallback
- Warns if input missing in selected environment
**Example**:
- Environment: `sepolia`
- Code: `api_key = input.mainnet_rpc`
- Result: Error if `mainnet_rpc` not in sepolia or global

---

### UC11: HCL Syntax Check

```mermaid
graph LR
    A[Content to<br/>validate] --> B[HCL Parser:<br/>parse_runbook]
    B --> C{Parse<br/>successful?}
    C -->|No| D[Extract error<br/>message + position]
    C -->|Yes| G[Return AST]
    D --> E[Convert to<br/>LSP Diagnostic]
    E --> F[Display syntax error<br/>in editor]
```

**Purpose**: Catch HCL syntax errors immediately
**Examples**:
- Missing closing braces
- Invalid attribute syntax
- Malformed strings
**Position Extraction**: Regex parsing of HCL error messages

---

### UC12: Run Linter Rules

```mermaid
graph TB
    AST[AST from<br/>HCL Parser] --> Linter[Linter Engine]

    Linter --> Rules[Execute Rules]

    subgraph "Active Rules"
        R1[undefined-input<br/>Check input references]
        R2[cli-override<br/>Warn on CLI overrides]
        R3[Type Validation<br/>Check action params]
        R4[Semantic Checks<br/>Action/signer validity]
    end

    Rules --> R1
    Rules --> R2
    Rules --> R3
    Rules --> R4

    R1 --> V1[Violations]
    R2 --> V1
    R3 --> V1
    R4 --> V1

    V1 --> Convert[Convert to<br/>LSP Diagnostics]
    Convert --> Severity{Violation<br/>level}
    Severity -->|Error| E[DiagnosticSeverity::ERROR]
    Severity -->|Warning| W[DiagnosticSeverity::WARNING]
    E --> Send[Send to editor]
    W --> Send
```

**Linter Rules**:
1. **undefined-input**: Checks input references against manifest + environment
2. **cli-override**: Warns when CLI inputs override environment values
3. **type-validation**: Validates action parameters match schemas
4. **semantic-validation**: Checks action types, signer references, etc.

**Integration**: `LinterValidationAdapter` bridges linter to LSP diagnostics

---

### UC13: Multi-file Validation

```mermaid
graph TB
    Start[Detect multi-file<br/>runbook] --> Check{Runbook<br/>location is<br/>directory?}
    Check -->|No| Single[Single-file<br/>validation]
    Check -->|Yes| MultiFlow

    MultiFlow --> Scan[FileScanner:<br/>find all .tx files<br/>in directory]
    Scan --> Sort[Sort files<br/>alphabetically]
    Sort --> Concat[Concatenate content<br/>with file markers]

    Concat --> Example["// File: action.tx\n...\n// File: signer.tx\n..."]

    Example --> BuildMap[Build line mapping<br/>line_num -> file_uri]
    BuildMap --> Validate[Validate combined<br/>content]
    Validate --> Results[Linter results]

    Results --> Map[Map diagnostics back<br/>to source files]
    Map --> Filter[Filter diagnostics<br/>for current file]
    Filter --> Return[Return diagnostics<br/>for displayed file]
```

**Purpose**: Support directory-based runbooks
**Example Structure**:
```
runbooks/
  my_runbook/
    actions.tx
    signers.sepolia.tx
    inputs.tx
```

**Process**:
1. Detect directory-based runbook in manifest
2. Load all `.tx` files in directory
3. Combine with file markers for position tracking
4. Validate as single unit
5. Map diagnostics back to original files
6. Return only diagnostics for current file

**Benefits**:
- Cross-file reference validation
- Consistent action/signer resolution
- Cleaner project organization

---

## Actor Descriptions

### Primary Actors

**Developer/User**
- Writes txtx runbooks
- Interacts through code editor
- Benefits from IDE features

**Code Editor** (VS Code, Neovim, etc.)
- Implements LSP client
- Displays diagnostics and UI
- Sends LSP requests

**Editor Extension/Plugin**
- Language-specific integration
- Custom UI (environment picker)
- Translates custom requests

### System Components

**LSP Server Core**
- Request router
- Handler orchestration
- Async task management

**Workspace State**
- Document cache
- Manifest cache
- Environment state

**Linter Engine**
- Rule execution
- Violation reporting
- Configurable rules

**HCL Parser**
- Syntax validation
- AST generation
- Error reporting

**Function Registry**
- Static function/action metadata
- Documentation lookup
- Signer type info

## Environment Context Flow

```mermaid
graph LR
    subgraph "Environment Lifecycle"
        A[Server Start] --> B{Env in<br/>init params?}
        B -->|Yes| C[Use provided env]
        B -->|No| D[Auto-detect env]
        D --> E{sepolia<br/>exists?}
        E -->|Yes| F[Use sepolia]
        E -->|No| G[Use first non-global]
        C --> H[Set current_environment]
        F --> H
        G --> H
        H --> I[All validations use<br/>this environment]
        I --> J[User switches env]
        J --> K[Re-validate all docs]
        K --> H
    end
```

## Summary of Use Cases

| Use Case | Actor | Trigger | Result |
|----------|-------|---------|--------|
| UC1: Open Document | Developer | Opens file | Document tracked + validated |
| UC2: Edit Document | Developer | Types in editor | Content synchronized + re-validated |
| UC3: Close Document | Developer | Closes file | Document removed from cache |
| UC4: Get Diagnostics | LSP Server | Document change | Errors/warnings displayed |
| UC5: Navigate to Definition | Developer | Ctrl+Click | Jump to manifest variable |
| UC6: View Hover Info | Developer | Hover over symbol | Popup with documentation/value |
| UC7: Get Completions | Developer | Types `input.` | Dropdown of available inputs |
| UC8: List Environments | Extension | Load workspace | Environment picker populated |
| UC9: Switch Environment | Developer | Selects from UI | All docs re-validated in context |
| UC10: Validate in Context | LSP Server | Environment set | Environment-aware checks |
| UC11: HCL Syntax Check | LSP Server | Parse document | Syntax error reporting |
| UC12: Run Linter Rules | LSP Server | Validate | Semantic error/warning reporting |
| UC13: Multi-file Validation | LSP Server | Directory runbook | Cross-file validation |

## Integration Points

```mermaid
graph TB
    subgraph "External Systems"
        Editor[Code Editor]
        FS[File System]
        Manifest[txtx.yml]
    end

    subgraph "LSP Server"
        Core[Server Core]
        Handlers[Request Handlers]
        State[Workspace State]
    end

    subgraph "Validation Pipeline"
        HCL[HCL Parser]
        Linter[Linter Engine]
        Rules[Rule Implementations]
    end

    Editor -->|JSON-RPC| Core
    Core -->|Dispatch| Handlers
    Handlers <-->|Read/Write| State
    State -->|Load| Manifest
    State -->|Read| FS
    Handlers --> HCL
    Handlers --> Linter
    Linter --> Rules
    Rules -->|Check| Manifest
```
