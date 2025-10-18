# Auto-generated from C4 annotations in Rust source code
# DO NOT EDIT - Regenerate with: just arch-c4
# For hand-written architecture including dynamic views, see workspace.dsl

workspace "txtx Validation Architecture (Generated from Code)" "Auto-generated from C4 annotations in Rust source" {

    model {
        user = person "Developer" "Writes txtx runbooks and manifests"

        txtxSystem = softwareSystem "txtx CLI" "Command-line tool for runbook execution and validation" {

            validation_core = container "Validation Core" "Container for Validation Core components" "Rust" {
                validation_result_types = component "Validation Result Types" "Defines core data structures for validation results" "Rust"
                // Responsibility: Define ValidationResult aggregating errors, warnings, suggestions
                // Responsibility: Track input references with line/column locations
                // Responsibility: Map error locations from combined files to original sources
                hcl_validation_visitor = component "HCL Validation Visitor" "Two-phase visitor pattern implementation for HCL validation" "Rust (hcl-edit visitor trait)"
                // Responsibility: Phase 1: Collect all block definitions and declarations
                // Responsibility: Phase 2: Validate references and dependency constraints
                // Responsibility: Extract and validate action parameters against specifications
                // Responsibility: Track input references for manifest validation
                runbook_validator = component "Runbook Validator" "High-level API for validating runbook files" "Rust"
                // Responsibility: Route validation to BasicHclValidator or FullHclValidator based on config
                // Responsibility: Manage addon specifications for validation
                validationcontext = component "ValidationContext" "Central state management for all validation operations" "Rust"
                // Responsibility: Manage validation state across all validation layers
                // Responsibility: Compute effective inputs from manifest + environment + CLI
                linter_rules = component "Linter Rules" "Custom validation rules for naming, security, and production requirements" "Rust"
                // Responsibility: Check input naming conventions (hyphens, uppercase)
                // Responsibility: Detect sensitive data patterns
                // Responsibility: Warn about CLI overrides and default values
                // Responsibility: Enforce production environment requirements
                dependency_graph = component "Dependency Graph" "Detects circular dependencies in variables and actions" "Rust (graph algorithms)"
                // Responsibility: Build dependency graphs from collected items
                // Responsibility: Find all cycles using depth-first search
                // Responsibility: Provide precise cycle paths for error reporting
                manifest_validator = component "Manifest Validator" "Validates runbook inputs against workspace manifests" "Rust"
                // Responsibility: Check that environment variables and inputs are properly defined
                // Responsibility: Validate input references against manifest environments
                hcl_diagnostics = component "HCL Diagnostics" "Extracts and converts HCL parse diagnostics" "Rust (hcl-edit)"
                // Responsibility: Extract diagnostics from HCL parse errors
                // Responsibility: Convert between byte offsets and line/column positions
                // Responsibility: Provide integrated diagnostic format for LSP/CLI
                hcl_validator = component "HCL Validator" "Validates HCL syntax, block structure, and references" "Rust (hcl-edit)"
                // Responsibility: Two-phase validation: collect definitions, then validate references
                // Responsibility: Detect circular dependencies in variables and actions
                // Responsibility: Validate action outputs, signers, variables, and flow inputs
                fileboundarymapper = component "FileBoundaryMapper" "Normalizes multi-file runbooks to single-file for validation" "Rust"
                // Responsibility: Track which lines in concatenated content belong to which files
                // Responsibility: Map error line numbers back to original source files
                rule_identification_system = component "Rule Identification System" "Type-safe rule identification for validation rules" "Rust"
                // Responsibility: Identify core and external validation rules
                // Responsibility: Determine rule applicability based on addon scope
                // Responsibility: Provide rule metadata (description, string representation)
                validation_helpers = component "Validation Helpers" "Shared validation utilities used across validation phases" "Rust"
                // Responsibility: Validate action namespace::action format
                // Responsibility: Look up and validate action specifications
                // Responsibility: Identify framework-specific inherited properties
                block_processors = component "Block Processors" "Processes individual HCL blocks during collection phase" "Rust (hcl-edit structure)"
                // Responsibility: Extract definitions from signer/variable/output blocks
                // Responsibility: Extract declarations from action/flow blocks
                // Responsibility: Build dependency information for cycle detection
            }

            runbook_core = container "Runbook Core" "Container for Runbook Core components" "Rust" {
                sourcelocationmapper = component "SourceLocationMapper" "Shared location tracking and span-to-position mapping" "Rust"
                // Responsibility: Track source locations (file, line, column) across the codebase
                // Responsibility: Convert byte offsets to line/column positions
                // Responsibility: Provide context about where references appear in HCL structure
            }
        }

        // Relationships
        hcl_validation_visitor -> dependency_graph "Uses"
        hcl_validation_visitor -> block_processors "Uses"
        hcl_validation_visitor -> validation_helpers "Uses"
        runbook_validator -> hcl_validator "Uses"
        validationcontext -> hcl_validator "Delegates to"
        validationcontext -> manifest_validator "Delegates to"
        sourcelocationmapper -> runbook_collector "Used by"
        sourcelocationmapper -> hcl_validator "Used by"
        sourcelocationmapper -> variable_extractor "Used by"
    }

    views {
        systemContext txtxSystem "SystemContext" {
            include *
            autoLayout lr
        }

        component validation_core {
            include *
            autoLayout tb
            title "Validation Core"
        }

        component runbook_core {
            include *
            autoLayout tb
            title "Runbook Core"
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
        }

        theme default
    }
}
