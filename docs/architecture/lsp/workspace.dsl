workspace "txtx LSP Architecture" "Real-time IDE integration for txtx runbooks" {

    model {
        developer = person "Developer" "Writes txtx runbooks in IDE"

        ide = softwareSystem "IDE/Editor" "VSCode, Neovim, etc." "External"

        txtxSystem = softwareSystem "txtx CLI" "Command-line tool with LSP server" {

            lspServer = container "LSP Server" "Real-time diagnostics and code intelligence" "Rust" {
                protocolHandler = component "Protocol Handler" "LSP message routing" "Rust"
                asyncHandler = component "AsyncLspHandler" "Concurrent request processing" "Rust"
                workspaceState = component "WorkspaceState" "Shared workspace state" "Rust"
                diagnosticsHandler = component "Diagnostics Handler" "Real-time validation" "Rust"
                completionHandler = component "Completion Handler" "Code completion" "Rust"
                hoverHandler = component "Hover Handler" "Hover documentation" "Rust"
                linterAdapter = component "Linter Adapter" "Reuses linter validation" "Rust"
            }

            validationCore = container "Validation Core" "Shared validation logic" "Rust (txtx-core)" {
                validationContext = component "ValidationContext" "Validation state" "Rust"
                hclValidator = component "HCL Validator" "Syntax and semantic validation" "Rust"
                manifestValidator = component "Manifest Validator" "Manifest validation" "Rust"
            }
        }

        # User interactions
        developer -> ide "Edits runbooks"
        ide -> protocolHandler "LSP requests" "JSON-RPC"
        diagnosticsHandler -> ide "Publishes diagnostics" "LSP Protocol"
        completionHandler -> ide "Returns completions" "LSP Protocol"
        hoverHandler -> ide "Returns hover info" "LSP Protocol"

        # LSP internal flow
        protocolHandler -> asyncHandler "Routes requests"
        asyncHandler -> workspaceState "Reads/updates state"
        asyncHandler -> diagnosticsHandler "textDocument/didChange"
        asyncHandler -> completionHandler "textDocument/completion"
        asyncHandler -> hoverHandler "textDocument/hover"

        # Validation flow
        diagnosticsHandler -> linterAdapter "Validate content"
        linterAdapter -> validationContext "Create context"
        validationContext -> hclValidator "Validate HCL"
        validationContext -> manifestValidator "Validate manifest"

        # Completion and hover
        completionHandler -> workspaceState "Get document + manifest"
        hoverHandler -> workspaceState "Get document context"

        # State management
        workspaceState -> workspaceState "Track open documents"
        workspaceState -> workspaceState "Cache manifest relationships"
    }

    views {
        systemContext txtxSystem "SystemContext" {
            include *
            autoLayout lr
            description "LSP server integrated into IDE workflow"
        }

        container txtxSystem "Containers" {
            include *
            autoLayout tb
            description "LSP Server and shared Validation Core"
        }

        component lspServer "LSPServer" {
            include *
            autoLayout tb
            description "LSP Server components"
        }

        dynamic lspServer "TextDocumentDidOpen" "Opening a runbook file in IDE" {
            developer -> ide "Opens runbook.tx"
            ide -> protocolHandler "textDocument/didOpen"
            protocolHandler -> asyncHandler "Route request"
            asyncHandler -> workspaceState "Store document content"
            asyncHandler -> diagnosticsHandler "Trigger validation"
            diagnosticsHandler -> linterAdapter "Validate"
            linterAdapter -> validationContext "Create context with manifest"
            validationContext -> hclValidator "Parse and validate HCL"
            hclValidator -> validationContext "Return errors"
            validationContext -> linterAdapter "Return validation result"
            linterAdapter -> diagnosticsHandler "Convert to diagnostics"
            diagnosticsHandler -> ide "publishDiagnostics"
            autoLayout lr
        }

        dynamic lspServer "TextDocumentDidChange" "Real-time validation on edit" {
            developer -> ide "Edits runbook"
            ide -> protocolHandler "textDocument/didChange"
            protocolHandler -> asyncHandler "Route request"
            asyncHandler -> workspaceState "Update document"
            asyncHandler -> diagnosticsHandler "Trigger validation"
            diagnosticsHandler -> linterAdapter "Validate (cached context)"
            linterAdapter -> validationContext "Use cached manifest"
            validationContext -> hclValidator "Incremental parse"
            hclValidator -> validationContext "Return errors"
            diagnosticsHandler -> ide "publishDiagnostics (<50ms)"
            autoLayout lr
        }

        dynamic lspServer "Completion" "Code completion for action names" {
            developer -> ide "Types 'action.' "
            ide -> protocolHandler "textDocument/completion"
            protocolHandler -> asyncHandler "Route with cache check"
            asyncHandler -> completionHandler "Get completions"
            completionHandler -> workspaceState "Get document + manifest"
            completionHandler -> ide "Return completion items"
            autoLayout lr
        }

        styles {
            element "Software System" {
                background #1168bd
                color #ffffff
            }
            element "Container" {
                background #438dd5
                color #ffffff
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "Person" {
                shape person
                background #08427b
                color #ffffff
            }
            element "External" {
                background #999999
                color #ffffff
            }
        }

        theme default
    }

}
