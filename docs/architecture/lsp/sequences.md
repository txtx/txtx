# txtx LSP Sequence Diagrams

This document contains sequence diagrams for all implemented LSP actions in the txtx Language Server.

## 1. Initialize & Server Capabilities

```mermaid
sequenceDiagram
    participant Client as LSP Client (Editor)
    participant Server as txtx LSP Server
    participant Workspace as WorkspaceState
    participant Handlers as Handler Registry

    Client->>Server: initialize(params)
    Note over Server: Extract root_uri and<br/>initialization options
    Server->>Server: Parse environment from<br/>initialization options
    Server->>Workspace: new()
    Workspace-->>Server: SharedWorkspaceState
    Server->>Handlers: new(workspace)
    Handlers-->>Server: Handlers instance

    alt Environment provided
        Server->>Workspace: set_environment(env)
    else No environment
        Server->>Workspace: get_environments()
        Workspace-->>Server: available_envs[]
        alt "sepolia" exists
            Server->>Workspace: set_environment("sepolia")
        else Use first non-global
            Server->>Workspace: set_environment(first_env)
        end
    end

    Server-->>Client: InitializeResult{<br/>  text_document_sync: FULL,<br/>  definition_provider: true,<br/>  hover_provider: true,<br/>  completion_provider: {<br/>    trigger_characters: ["."]<br/>  }<br/>}
    Client->>Server: initialized notification
    Note over Server,Client: Server ready to accept requests
```

## 2. Document Lifecycle (didOpen/didChange/didClose)

```mermaid
sequenceDiagram
    participant Client as LSP Client
    participant Server as LSP Server
    participant DocSync as DocumentSyncHandler
    participant Workspace as WorkspaceState
    participant Diag as DiagnosticsHandler
    participant Linter as Linter Integration
    participant HCL as HCL Parser

    %% Document Open
    Client->>Server: textDocument/didOpen
    Server->>DocSync: did_open(params)
    DocSync->>Workspace: open_document(uri, content)
    Workspace->>Workspace: Store document v1

    Server->>Diag: get_diagnostics(uri)
    Diag->>Workspace: get_document(uri)
    Workspace-->>Diag: Document

    alt Is Runbook
        Diag->>Workspace: get_manifest_for_document(uri)
        Workspace-->>Diag: Manifest

        alt Multi-file runbook
            Diag->>Diag: validate_with_multi_file_support()
            Diag->>Linter: load_multi_file_runbook()
            Diag->>Linter: validate_content()
        else Single file
            Diag->>HCL: parse_runbook()
            HCL-->>Diag: syntax errors
            Diag->>Linter: validate_content()
        end

        Linter-->>Diag: ValidationResult
        Diag->>Diag: Convert to LSP Diagnostics
    end

    Diag-->>Server: Diagnostic[]
    Server->>Client: textDocument/publishDiagnostics

    %% Document Change
    Client->>Server: textDocument/didChange
    Server->>DocSync: did_change(params)
    DocSync->>Workspace: update_document(uri, new_content)
    Workspace->>Workspace: Increment version, update content

    Server->>Diag: get_diagnostics(uri)
    Note over Diag,Linter: Same validation flow as didOpen
    Server->>Client: textDocument/publishDiagnostics

    %% Document Close
    Client->>Server: textDocument/didClose
    Server->>DocSync: did_close(params)
    DocSync->>Workspace: close_document(uri)
    Workspace->>Workspace: Remove document from cache
```

## 3. Go to Definition

```mermaid
sequenceDiagram
    participant Client as LSP Client
    participant Server as LSP Server
    participant DefHandler as EnhancedDefinitionHandler
    participant Workspace as WorkspaceState

    Client->>Server: textDocument/definition<br/>{uri, position}
    Server->>DefHandler: goto_definition(params)
    DefHandler->>DefHandler: get_document_at_position(params)
    DefHandler->>Workspace: read()
    Workspace-->>DefHandler: WorkspaceState
    DefHandler->>Workspace: get_document(uri)
    Workspace-->>DefHandler: Document{content, version}

    DefHandler->>DefHandler: extract_input_reference(content, position)
    Note over DefHandler: Regex match: input\.(\w+)<br/>Check cursor within match bounds

    alt Input reference found
        DefHandler->>Workspace: get_manifest_for_runbook(uri)
        Workspace-->>DefHandler: Manifest
        DefHandler->>DefHandler: find_variable_line(manifest_uri, var_ref)
        Note over DefHandler: Search manifest YAML<br/>for variable definition

        alt Variable found
            DefHandler-->>Server: Location{<br/>  uri: manifest_uri,<br/>  range: {line, 0} to {line, 100}<br/>}
        else Not found
            DefHandler-->>Server: None
        end
    else No reference
        DefHandler-->>Server: None
    end

    Server-->>Client: GotoDefinitionResponse
```

## 4. Hover Information

```mermaid
sequenceDiagram
    participant Client as LSP Client
    participant Server as LSP Server
    participant HoverHandler as HoverHandler
    participant Workspace as WorkspaceState
    participant Functions as Function Registry
    participant EnvResolver as EnvironmentResolver

    Client->>Server: textDocument/hover<br/>{uri, position}
    Server->>HoverHandler: hover(params)
    HoverHandler->>HoverHandler: get_document_at_position(params)

    %% Try function/action hover
    HoverHandler->>HoverHandler: try_function_or_action_hover()
    HoverHandler->>HoverHandler: extract_function_or_action(content, position)
    Note over HoverHandler: Check if in comment<br/>Regex: (\w+)::([\w_]+)

    alt Function/Action/Signer found
        HoverHandler->>Functions: get_function_hover(reference)
        alt Function found
            Functions-->>HoverHandler: Function documentation
            HoverHandler-->>Server: Hover{markdown content}
        else Not function
            HoverHandler->>Functions: get_action_hover(reference)
            alt Action found
                Functions-->>HoverHandler: Action documentation
                HoverHandler-->>Server: Hover{markdown content}
            else Not action
                HoverHandler->>Functions: get_signer_hover(reference)
                alt Static signer found
                    Functions-->>HoverHandler: Signer documentation
                else Environment signer (namespace::name)
                    HoverHandler->>Workspace: get_current_environment()
                    HoverHandler->>HoverHandler: Generate generic signer hover
                    HoverHandler-->>Server: Hover{environment-specific info}
                end
            end
        end
    end

    %% Try input hover
    HoverHandler->>HoverHandler: try_input_hover()
    HoverHandler->>HoverHandler: extract_input_reference(content, position)

    alt Input reference found
        alt Special debug command (dump_txtx_state)
            HoverHandler->>HoverHandler: debug_handler.dump_state(uri)
        else Regular input
            HoverHandler->>Workspace: get_current_environment()
            HoverHandler->>Workspace: get_manifest_for_document(uri)
            Workspace-->>HoverHandler: Manifest
            HoverHandler->>EnvResolver: new(manifest, current_env)
            HoverHandler->>EnvResolver: resolve_value(var_ref)

            alt Value found
                EnvResolver-->>HoverHandler: (value, source_env)
                HoverHandler->>EnvResolver: get_all_values(var_ref)
                EnvResolver-->>HoverHandler: Map<env_name, value>
                HoverHandler->>HoverHandler: Build hover text with:<br/>- Current value<br/>- Source environment<br/>- Other definitions
            else Not found in current env
                HoverHandler->>EnvResolver: get_all_values(var_ref)
                alt Defined elsewhere
                    HoverHandler->>HoverHandler: Show warning + available envs
                else Not defined anywhere
                    HoverHandler->>HoverHandler: Show error + suggestion
                end
            end

            HoverHandler-->>Server: Hover{markdown content}
        end
    end

    Server-->>Client: Hover | null
```

## 5. Code Completion

```mermaid
sequenceDiagram
    participant Client as LSP Client
    participant Server as LSP Server
    participant AsyncHandler as AsyncLspHandler
    participant CompHandler as CompletionHandler
    participant Workspace as WorkspaceState

    Note over Server: Heavy operation - runs async

    Client->>Server: textDocument/completion<br/>{uri, position, trigger}
    Server->>Server: spawn_async_task()
    Server->>AsyncHandler: handle_request(req)
    AsyncHandler->>CompHandler: completion(params)
    CompHandler->>CompHandler: get_document_at_position(params)
    CompHandler->>Workspace: read()
    Workspace-->>CompHandler: WorkspaceState
    CompHandler->>Workspace: get_document(uri)
    Workspace-->>CompHandler: Document

    CompHandler->>CompHandler: is_after_input_dot(content, position)
    Note over CompHandler: Check if cursor follows "input."<br/>Look back 6 chars from position

    alt After "input."
        CompHandler->>Workspace: get_manifest_for_runbook(uri)
        Workspace-->>CompHandler: Manifest

        loop For each environment
            CompHandler->>CompHandler: Collect input keys
        end

        CompHandler->>CompHandler: Build CompletionItem[]<br/>kind: VARIABLE
        CompHandler-->>AsyncHandler: CompletionResponse::Array(items)
    else Not after "input."
        CompHandler-->>AsyncHandler: None
    end

    AsyncHandler-->>Server: Response
    Server-->>Client: CompletionList | null
```

## 6. Environment Management (Custom)

```mermaid
sequenceDiagram
    participant Client as LSP Client/Extension
    participant Server as LSP Server
    participant WSHandler as WorkspaceHandler
    participant Workspace as WorkspaceState
    participant FileScanner as FileScanner
    participant DiagHandler as DiagnosticsHandler

    %% Get Environments
    Client->>Server: workspace/environments (custom request)
    Server->>WSHandler: get_environments()
    WSHandler->>WSHandler: collect_environments_from_documents()
    WSHandler->>Workspace: read()
    WSHandler->>Workspace: documents()

    loop For each document URI
        WSHandler->>WSHandler: extract_environment_from_uri(uri)
        Note over WSHandler: Parse *.{env}.tx pattern
    end

    WSHandler->>WSHandler: collect_environments_from_manifest()
    WSHandler->>Workspace: get_manifest_for_document()
    Note over WSHandler: Extract environments.keys()

    alt Few environments found
        WSHandler->>WSHandler: scan_workspace_for_environments()
        WSHandler->>FileScanner: find_tx_files(workspace_root)
        FileScanner-->>WSHandler: tx_files[]
        loop For each file
            WSHandler->>WSHandler: extract_environment_from_path(file)
        end
    end

    WSHandler->>WSHandler: Filter out "global"<br/>Sort results
    WSHandler-->>Server: env_list[]
    Server-->>Client: ["sepolia", "mainnet", ...]

    %% Set Environment
    Client->>Server: workspace/setEnvironment<br/>{environment: "sepolia"}
    Server->>WSHandler: set_environment("sepolia")
    WSHandler->>Workspace: write()
    WSHandler->>Workspace: set_current_environment(Some("sepolia"))

    %% Re-validate all documents
    Server->>Workspace: read()
    Server->>Workspace: documents().keys()
    Workspace-->>Server: document_uris[]

    loop For each open document
        Server->>DiagHandler: get_diagnostics_with_env(uri, "sepolia")
        DiagHandler->>DiagHandler: Validate with new environment
        DiagHandler-->>Server: Diagnostic[]
        Server->>Client: textDocument/publishDiagnostics
    end
```

## 7. Diagnostics with Linter Integration

```mermaid
sequenceDiagram
    participant Diag as DiagnosticsHandler
    participant Validator as LinterValidationAdapter
    participant Linter as Linter
    participant Rules as Linter Rules
    participant HCL as HCL Parser
    participant MultiFile as MultiFile Support

    Diag->>Validator: validate_document(uri, content, manifest)

    %% Create Linter
    Validator->>Validator: Create LinterConfig{<br/>  manifest_path,<br/>  environment,<br/>  cli_inputs,<br/>  format: Json<br/>}
    Validator->>Linter: new(config)

    alt Linter creation fails
        Validator-->>Diag: ERROR diagnostic
    end

    %% Multi-file detection
    alt Multi-file runbook
        Validator->>MultiFile: load_multi_file_runbook(runbook_name)
        MultiFile->>MultiFile: Scan directory for *.tx files
        MultiFile->>MultiFile: Concatenate files with markers
        MultiFile-->>Validator: (combined_content, file_map)
    end

    %% Validation
    Validator->>Linter: validate_content(content, file_path, manifest_path, env)

    Linter->>HCL: parse_runbook(content)

    alt Parse error
        HCL-->>Linter: HCL syntax errors
        Linter->>Linter: Convert to ValidationOutcome
    else Parse success
        HCL-->>Linter: AST

        loop For each rule
            Linter->>Rules: check(ast, manifest, environment)
            Rules->>Rules: Visit AST nodes
            Rules->>Rules: Check semantics
            Rules-->>Linter: Violations[]
        end
    end

    Linter-->>Validator: ValidationResult{<br/>  errors: [],<br/>  warnings: []<br/>}

    %% Convert to LSP diagnostics
    loop For each error
        Validator->>Validator: Create Diagnostic{<br/>  severity: ERROR,<br/>  range: {line, column},<br/>  source: "txtx-linter"<br/>}
    end

    loop For each warning
        Validator->>Validator: Create Diagnostic{<br/>  severity: WARNING,<br/>  range: {line, column},<br/>  source: "txtx-linter"<br/>}
    end

    alt Multi-file
        Validator->>MultiFile: map_line_to_file(diagnostic.line, file_map)
        MultiFile-->>Validator: (original_file_uri, adjusted_line)
        Note over Validator: Only return diagnostics<br/>for current file
    end

    Validator-->>Diag: Diagnostic[]
```

## Key Components Summary

### Handlers
- **DocumentSyncHandler**: Manages document lifecycle (open/change/close)
- **EnhancedDefinitionHandler**: Go-to-definition for inputs
- **HoverHandler**: Context-aware hover with function/action/input info
- **CompletionHandler**: Auto-completion for inputs after "input."
- **DiagnosticsHandler**: Real-time validation with linter rules
- **WorkspaceHandler**: Environment management (custom protocol)

### Validation Flow
1. **HCL Parser**: Syntax validation
2. **Linter Rules**: Semantic validation (undefined-input, cli-override, etc.)
3. **Multi-file Support**: Handles directory-based runbooks
4. **Environment Context**: Validates against selected environment

### Async Operations
- Completion and hover requests run in Tokio runtime
- Heavy operations don't block main LSP thread
- Results sent back via channel

### State Management
- **SharedWorkspaceState**: Thread-safe `Arc<RwLock<WorkspaceState>>`
- Tracks open documents with versions
- Caches parsed manifests
- Maintains current environment selection
