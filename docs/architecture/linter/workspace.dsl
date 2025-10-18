workspace "txtx Linter Architecture" "Static analysis and validation for txtx runbooks" {

    model {
        user = person "Developer" "Writes txtx runbooks and manifests"

        txtxSystem = softwareSystem "txtx CLI" "Command-line tool for runbook execution and validation" {

            lintCommand = container "Lint Command" "CLI entry point for validation" "Rust" {
                cliInterface = component "CLI Interface" "Parses user commands and arguments" "Rust" {
                    tags "UserInterface"
                }
                workspaceAnalyzer = component "WorkspaceAnalyzer" "Discovers manifests and resolves runbooks" "Rust"
                linterEngine = component "Linter Engine" "Orchestrates validation pipeline" "Rust"
                formatter = component "Formatter" "Formats validation results" "Rust" {
                    tags "Formatter"
                }
                output = component "Output Handler" "Displays results to user" "Rust" {
                    tags "UserInterface"
                }
            }

            validationCore = container "Validation Core" "Core validation logic" "Rust (txtx-core)" {
                validationContext = component "ValidationContext" "Central validation state" "Rust"
                hclValidator = component "HCL Validator" "Syntax and semantic validation" "Rust"
                manifestValidator = component "Manifest Validator" "Environment and input validation" "Rust"
                fileBoundaryMapper = component "FileBoundaryMapper" "Maps errors to source files" "Rust"
            }

            linterRules = container "Linter Rules" "Style and convention checks" "Rust (txtx-cli)" {
                undefinedInput = component "undefined-input" "Check inputs exist in manifest" "Rust Rule"
                namingConvention = component "naming-convention" "Check naming style" "Rust Rule"
                cliOverride = component "cli-override" "Warn about CLI overrides" "Rust Rule"
                sensitiveData = component "sensitive-data" "Suggest vault usage" "Rust Rule"
            }

            lspServer = container "LSP Server" "Real-time IDE diagnostics" "Rust" {
                diagnosticsHandler = component "Diagnostics Handler" "Provides real-time validation" "Rust"
            }
        }

        ideSystem = softwareSystem "IDE/Editor" "VSCode, Neovim, etc." "External"

        # Relationships - User interactions
        user -> cliInterface "Runs: txtx lint runbook.tx"
        cliInterface -> workspaceAnalyzer "Parse args, discover workspace"
        user -> ideSystem "Edits runbooks"
        ideSystem -> lspServer "Requests diagnostics" "LSP Protocol"
        formatter -> output "Send formatted results"
        output -> user "Display errors/warnings"

        # Relationships - Lint Command flow
        workspaceAnalyzer -> linterEngine "Provides runbook and manifest"
        linterEngine -> validationContext "Creates with config"
        validationContext -> hclValidator "Delegates HCL validation"
        validationContext -> manifestValidator "Delegates manifest validation"
        manifestValidator -> linterRules "Runs lint rules"
        linterEngine -> fileBoundaryMapper "Maps multi-file errors" "For multi-file runbooks"
        linterEngine -> formatter "Formats results"

        # Relationships - LSP flow
        diagnosticsHandler -> linterEngine "Reuses linter logic"

        # Validation flow details
        hclValidator -> hclValidator "Parse AST, visit nodes"
        manifestValidator -> manifestValidator "Extract refs, check definitions"

        # Multi-file specific
        fileBoundaryMapper -> fileBoundaryMapper "Track file boundaries during concatenation"
    }

    views {
        systemContext txtxSystem "SystemContext" {
            include *
            autoLayout lr
            description "System context diagram showing txtx and its users"
        }

        container txtxSystem "Containers" {
            include *
            autoLayout lr
            description "Container diagram showing major components"
        }

        component lintCommand "LintCommand" {
            include *
            autoLayout tb
            description "Lint command components"
        }

        component validationCore "ValidationCore" {
            include *
            autoLayout tb
            description "Core validation components"
        }

        component linterRules "LinterRules" {
            include *
            autoLayout lr
            description "Individual linter rules"
        }

        dynamic lintCommand "SingleFileValidation" "Single file validation flow" {
            cliInterface -> workspaceAnalyzer "txtx lint runbook.tx"
            workspaceAnalyzer -> linterEngine "Load runbook + manifest"
            linterEngine -> validationContext "Create context"
            validationContext -> hclValidator "Validate syntax"
            hclValidator -> validationContext "Return HCL errors"
            validationContext -> manifestValidator "Validate manifest"
            manifestValidator -> validationContext "Return manifest errors"
            validationContext -> linterEngine "Return all errors"
            linterEngine -> formatter "Format results"
            formatter -> output "Stylish/JSON/Compact/Quickfix"
            autoLayout lr
        }

        dynamic lintCommand "MultiFileValidation" "Multi-file runbook validation with boundary mapping" {
            cliInterface -> workspaceAnalyzer "txtx lint flows/"
            workspaceAnalyzer -> linterEngine "Load multi-file runbook"
            linterEngine -> fileBoundaryMapper "Track: flows.tx (lines 1-10)"
            linterEngine -> fileBoundaryMapper "Track: deploy.tx (lines 11-25)"
            linterEngine -> fileBoundaryMapper "Concatenate all files"
            linterEngine -> validationContext "Validate combined content"
            validationContext -> manifestValidator "Check flow inputs"
            manifestValidator -> validationContext "Error at line 18 (combined)"
            validationContext -> linterEngine "Return errors"
            linterEngine -> fileBoundaryMapper "Map line 18 → deploy.tx:8"
            linterEngine -> formatter "Format with accurate locations"
            formatter -> output "deploy.tx:8:1 (not line 18)"
            autoLayout lr
        }

        dynamic lintCommand "FlowValidation" "Flow validation with related locations" {
            cliInterface -> workspaceAnalyzer "txtx lint flows/"
            workspaceAnalyzer -> linterEngine "Load: flows.tx + deploy.tx"
            linterEngine -> validationContext "Validate combined"
            validationContext -> manifestValidator "Check flow inputs"
            manifestValidator -> manifestValidator "Collect: flow definitions from flows.tx"
            manifestValidator -> manifestValidator "Collect: flow.* refs from deploy.tx"
            manifestValidator -> manifestValidator "Partition: flows missing input"
            manifestValidator -> validationContext "Error with related_locations"
            validationContext -> linterEngine "Return flow errors"
            linterEngine -> fileBoundaryMapper "Map both locations"
            linterEngine -> formatter "Format with related locs"
            formatter -> output "flows.tx:5 + deploy.tx:11"
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
            element "Formatter" {
                background #f4a261
            }
            element "UserInterface" {
                background #06d6a0
                color #000000
            }
        }

        theme default
    }

}
