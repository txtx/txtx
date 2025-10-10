# txtx Linter and LSP Features Documentation

This document explains the behavior of txtx's linter and Language Server Protocol (LSP) features, including scoping rules for references and rename operations.

## Table of Contents

1. [Reference Scoping](#reference-scoping)
2. [Rename Scoping](#rename-scoping)
3. [Linter Overview](#linter-overview)
4. [LSP Features](#lsp-features)

## Reference Scoping

The LSP's "Find References" feature respects different scoping rules depending on the reference type.

### Workspace-Scoped References

These reference types can be used across **all runbooks** in the workspace:

- **`input.*`** - Inputs defined in the manifest's `environments` section
- **`signer.*`** - Signers that can be shared across runbooks

**Example:**

```yaml
# txtx.yml
environments:
  global:
    api_key: "default_key"
```

Finding references to `input.api_key` from any runbook will show **all** uses across:

- All runbook files (regardless of which runbook they belong to)
- The manifest file itself

### Runbook-Scoped References

These reference types are **local to a single runbook**:

- **`variable.*`** - Variables defined within a runbook
- **`flow.*`** - Flows defined within a runbook
- **`action.*`** - Actions defined within a runbook
- **`output.*`** - Outputs defined within a runbook

**Example:**

```yaml
# txtx.yml
runbooks:
  - name: deploy
    location: deploy/
  - name: monitor
    location: monitor/
```

```hcl
// deploy/flows.tx
variable "network_id" {
  value = "1"
}

// monitor/main.tx
variable "network_id" {
  value = "2"
}
```

Finding references to `variable.network_id` from `deploy/flows.tx` will **only** show uses in the `deploy` runbook files, not from the `monitor` runbook.

### Special Case: Files Without Runbooks

Files that are not part of any runbook (loose files in the workspace root) are treated as **workspace-wide**. References in these files are searched globally.

## Rename Scoping

The LSP's "Rename Symbol" feature uses **exactly the same scoping rules** as "Find References":

- **Workspace-scoped** types (`input`, `signer`) - Renamed across all runbooks
- **Runbook-scoped** types (`variable`, `flow`, `action`, `output`) - Renamed only within the current runbook

This ensures consistency: if "Find References" shows you 5 locations, "Rename" will update those same 5 locations.

### Cross-File Rename Examples

#### Example 1: Renaming a Workspace-Scoped Input

```yaml
# txtx.yml
environments:
  global:
    api_key: "secret"  # ← Will be renamed
```

```hcl
// deploy/main.tx
action "call_api" {
  url = input.api_key  # ← Will be renamed
}

// monitor/check.tx
action "monitor" {
  key = input.api_key  # ← Will be renamed
}
```

Renaming `input.api_key` → `input.api_token` will update **all 3 locations** across different runbooks.

#### Example 2: Renaming a Runbook-Scoped Variable

```hcl
// deploy/variables.tx (runbook: deploy)
variable "network_id" {
  value = "1"  # ← Will be renamed
}

// deploy/actions.tx (runbook: deploy)
action "deploy" {
  network = variable.network_id  # ← Will be renamed
}

// monitor/main.tx (runbook: monitor)
variable "network_id" {
  value = "2"  # ← Will NOT be renamed (different runbook)
}
```

Renaming `variable.network_id` → `variable.chain_id` from `deploy/variables.tx` will update **only the deploy runbook** files, leaving the monitor runbook unchanged.

## Linter Overview

The txtx linter validates runbook syntax and semantics before execution.

### Linter Rules

The linter implements various validation rules, including:

- **`undefined-variable`** - Detects references to undefined variables
- **`undefined-input`** - Detects references to inputs not defined in the manifest
- **`cli-override`** - Warns when CLI inputs may override manifest environment values
- **`cyclic-dependency`** - Detects circular dependencies between definitions
- **`type-mismatch`** - Validates type compatibility in expressions

### CLI Override Rule

The `cli-override` rule warns when a CLI input (`--input var=value`) might override a value defined in the manifest's environment.

**Important:** txtx does NOT read OS environment variables (like `$PATH`, `$HOME`). It uses a manifest-based environment system.

#### How txtx Environments Work

1. **Manifest-Based**: All inputs are defined in `txtx.yml`
2. **Environment Selection**: Environments (dev, staging, production) are defined in the manifest
3. **Global Defaults**: The `global` environment provides default values

#### Input Resolution Precedence

txtx resolves input values using this hierarchy (highest to lowest priority):

1. **CLI inputs** (`--input var=value`) - Directly specified on command line
2. **txtx environment** (`--env production`) - Environment-specific values from manifest
3. **txtx global environment** - Default values in `environments.global`

## LSP Features

### Supported Features

1. **Go to Definition** - Jump from a reference to its definition
   - Respects runbook scoping for runbook-scoped types
   - Works across files for workspace-scoped types
   - **Flow field navigation**: `flow.chain_id` shows all flows with `chain_id` field

2. **Find References** - Find all uses of a symbol
   - Workspace-scoped: Searches all runbooks
   - Runbook-scoped: Searches only current runbook

3. **Rename Symbol** - Rename a symbol across files
   - Uses same scoping rules as Find References
   - Atomic: all-or-nothing rename operation

4. **Hover** - Show documentation and type information

5. **Completion** - Auto-complete for available symbols
   - Suggests inputs from manifest
   - Suggests variables/flows/actions from current runbook

6. **Diagnostics** - Real-time error and warning feedback
   - Multi-file runbook validation
   - Shows errors from all files, not just open buffers

### Flow Field Navigation

The LSP supports intelligent navigation for flow field access patterns like `flow.chain_id`.

When you use "Go to Definition" on the field name in `flow.field_name`, the LSP finds all flows that define that field:

**Example:**

```hcl
// flows.tx
flow "super1" {
  chain_id = "11155111"
}

flow "super2" {
  chain_id = "2"
}

// deploy.tx
action "deploy" {
  constructor_args = [
    flow.chain_id  // ← Go-to-definition shows both super1 and super2
  ]
}
```

**Behavior:**

- **Single match**: Jump directly to the flow definition
- **Multiple matches**: Show location picker with all flows that have the field
- **Scoping**: Respects runbook boundaries (only shows flows from current runbook)
- **No match**: Returns no definition found

This allows you to quickly discover which flows provide a particular field, making it easy to understand the available flow configurations.

### Multi-File Runbooks

txtx supports multi-file runbooks where a single runbook is split across multiple `.tx` files in a directory:

```yaml
# txtx.yml
runbooks:
  - name: deploy
    location: deploy/  # Directory containing multiple .tx files
```

```
deploy/
├── flows.tx       # Flow definitions
├── variables.tx   # Variable definitions
├── actions.tx     # Action definitions
└── outputs.tx     # Output definitions
```

**LSP Behavior:**

- Diagnostics show errors from **all files** in the runbook (even unopened files)
- References and rename work across all files in the runbook
- Go-to-definition navigates between files seamlessly

### Editor Support

The LSP is language-agnostic and works with:

- **VS Code** - via txtx extension
- **Neovim** - via nvim-txtx plugin
- **Any LSP-compatible editor** - via `txtx lsp` command

## Testing

The implementation includes comprehensive tests for all scoping scenarios:

1. **Variable references scoped to single runbook** - Variables with same name in different runbooks don't cross-reference
2. **Flow references stay within runbook boundary** - Flows are local to their runbook
3. **Input references cross all runbooks** - Inputs are workspace-wide
4. **Action/Output references scoped to runbook** - Actions and outputs are runbook-local
5. **Files without runbook are workspace-wide** - Loose files search globally

Run tests:

```bash
cargo test-cli-unit -- references_test rename
```
